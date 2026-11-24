use crate::{Application, configuration::RateLimitConfiguration, dependency_injection::CfgRequire, http_context::HttpContext, macros::middleware, middleware::MiddlewareNext};
use dashmap::DashMap;
use std::{
    collections::VecDeque,
    net::IpAddr,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::task::JoinHandle;

#[derive(Default)]
struct ClientWindow {
    requests: VecDeque<Instant>,
    blocked_until: Option<Instant>,
}

impl ClientWindow {
    fn is_active(&self, now: Instant, window: Duration) -> bool {
        self.blocked_until.is_some_and(|until| now < until) || self.requests.back().is_some_and(|last| now.duration_since(*last) <= window)
    }

    fn admit(&mut self, now: Instant, window: Duration, max_requests: usize, block_duration: Duration) -> bool {
        if self.blocked_until.is_some_and(|until| now < until) {
            return false;
        }
        self.blocked_until = None;
        while self.requests.front().is_some_and(|oldest| now.duration_since(*oldest) > window) {
            self.requests.pop_front();
        }
        if self.requests.len() >= max_requests {
            self.requests.clear();
            self.blocked_until = Some(now + block_duration);
            return false;
        }
        self.requests.push_back(now);
        true
    }
}

pub struct RateLimitMiddleware {
    next: MiddlewareNext,
    clients: Arc<DashMap<IpAddr, ClientWindow>>,
    cleanup: JoinHandle<()>,
    max_requests: usize,
    window: Duration,
    block_duration: Duration,
}
#[middleware]
impl RateLimitMiddleware {
    pub fn new(next: MiddlewareNext, config: CfgRequire<RateLimitConfiguration>) -> Self {
        let window = Duration::from_secs(config.limit_seconds.into());
        let clients = Arc::new(DashMap::<IpAddr, ClientWindow>::new());
        let cleanup_clients = clients.clone();
        let cleanup = tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(60)).await;
                let now = Instant::now();
                cleanup_clients.retain(|_, client| client.is_active(now, window));
            }
        });
        Self {
            next,
            clients,
            cleanup,
            max_requests: config.max_requests,
            window,
            block_duration: Duration::from_secs(config.block_duration_seconds.into()),
        }
    }
    pub async fn invoke_async(&self, context: &mut HttpContext) {
        let admitted = {
            let mut client = self.clients.entry(context.request.client_socket_addr.ip()).or_default();
            client.admit(Instant::now(), self.window, self.max_requests, self.block_duration)
        };
        if !admitted {
            context.response.status_code = http::StatusCode::TOO_MANY_REQUESTS;
            context.response.body = b"Too Many Requests".to_vec();
            return;
        }
        self.next.next_async(context).await;
    }
}

impl Drop for RateLimitMiddleware {
    fn drop(&mut self) {
        self.cleanup.abort();
    }
}

impl Application {
    /// Limit the number of requests from a single IP address within a specified time window.
    pub fn use_rate_limit(&mut self) -> &mut Self {
        self.add_middleware::<RateLimitMiddleware>();
        self
    }
}
