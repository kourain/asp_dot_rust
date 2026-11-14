use crate::{Application, configuration::RateLimitConfiguration, dependency_injection::CfgRequire, http_context::HttpContext, macros::middleware, middleware::MiddlewareNext};
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
    pub fn new(next: MiddlewareNext, config: CfgRequire<RateLimitConfiguration>) -> Self {
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
        let client_ip = context.request.client_socket_addr.ip();
        let now = std::time::Instant::now();

        // Reject while still blocked.
        if let Some(blocked_until) = self.ip_blocked_until.get(&client_ip) {
            if now < *blocked_until {
                context.response.status_code = http::StatusCode::TOO_MANY_REQUESTS;
                context.response.body = http::StatusCode::TOO_MANY_REQUESTS.canonical_reason().unwrap_or("Too Many Requests").as_bytes().to_vec();
                return;
            }
            drop(blocked_until);
            self.ip_blocked_until.remove(&client_ip);
        }

        let window = std::time::Duration::from_secs(self.limit_seconds.into());
        let mut history = self.ip_request_counts.entry(client_ip).or_insert_with(Vec::new);
        // Drop timestamps outside the sliding window.
        history.retain(|t| now.duration_since(*t) <= window);
        history.push(now);

        if history.len() > self.max_requests {
            self.ip_blocked_until.insert(client_ip, now + std::time::Duration::from_secs(self.block_duration_seconds.into()));
            history.clear();
            context.response.status_code = http::StatusCode::TOO_MANY_REQUESTS;
            context.response.body = http::StatusCode::TOO_MANY_REQUESTS.canonical_reason().unwrap_or("Too Many Requests").as_bytes().to_vec();
            return;
        }

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
