use crate::{Application, configuration::RateLimitConfiguration, dependency_injection::Cfg, http_context::HttpContext, macros::middleware, middleware::MiddlewareNext};
use dashmap::DashMap;

pub struct RateLimitMiddleware {
    next: MiddlewareNext,
    ip_request_counts: DashMap<std::net::IpAddr, Vec<std::time::Instant>>,
    ip_blocked_until: DashMap<std::net::IpAddr, std::time::Instant>,
    max_requests: usize,
    limit_seconds: u32,
    block_duration_seconds: u32,
}
#[middleware]
impl RateLimitMiddleware {
    pub fn new(next: MiddlewareNext, config: Cfg<RateLimitConfiguration>) -> Self {
        let config = config.unwrap_or_default();
        RateLimitMiddleware {
            next,
            ip_request_counts: DashMap::new(),
            ip_blocked_until: DashMap::new(),
            max_requests: config.max_requests,
            limit_seconds: config.limit_seconds,
            block_duration_seconds: config.block_duration_seconds,
        }
    }
    pub async fn invoke_async(&self, context: &mut HttpContext) {
        let client_socket_addr = context.request.client_socket_addr;
        let now = std::time::Instant::now();
        match self.ip_blocked_until.get(&client_socket_addr.ip()) {
            Some(blocked_time) => {
                if now < *blocked_time {
                    context.response.status_code = http::StatusCode::TOO_MANY_REQUESTS; // Too Many Requests
                    context.response.body = http::StatusCode::TOO_MANY_REQUESTS.canonical_reason().unwrap_or("Too Many Requests").as_bytes().to_vec();
                    return;
                } else {
                    self.ip_blocked_until.remove(&client_socket_addr.ip());
                }
            }
            None => {
                self.ip_blocked_until.insert(client_socket_addr.ip(), std::time::Instant::now());
            }
        };
        match self.ip_request_counts.get_mut(&client_socket_addr.ip()) {
            Some(mut history_access) => {
                history_access.push(now);
                let mut index: usize = 0;
                for i in (0..history_access.len()).rev() {
                    if now.duration_since(history_access[i]) > std::time::Duration::from_secs(self.limit_seconds.into()) {
                        index = i;
                        break;
                    }
                }
                if history_access.len() - index - 1 > self.max_requests {
                    self.ip_blocked_until
                        .insert(client_socket_addr.ip(), std::time::Instant::now() + std::time::Duration::from_secs(self.block_duration_seconds.into()));
                    history_access.clear(); // Clear history to start fresh after blocking
                } else {
                    history_access.drain(..index); // Remove old timestamps outside the time window
                }
            }
            None => {
                self.ip_request_counts.insert(client_socket_addr.ip(), vec![now]);
            }
        };

        self.next.next_async(context).await;
    }
}

impl Application {
    /// Limit the number of requests from a single IP address within a specified time window.
    pub fn use_rate_limit(&mut self) -> &mut Self {
        self.add_middleware::<RateLimitMiddleware>();
        self
    }
}
