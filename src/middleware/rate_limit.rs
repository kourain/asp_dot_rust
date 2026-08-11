use crate::{
    Application,
    configuration::RateLimitConfiguration,
    dependcy_injection::DependcyInjectableService,
    http_context::HttpContext,
    middleware::{Middleware, MiddlewareNext},
    services::configuration::ConfigurationService,
};
use dashmap::DashMap;

pub struct RateLimitMiddleware {
    ip_request_counts: DashMap<std::net::IpAddr, Vec<std::time::Instant>>,
    ip_blocked_until: DashMap<std::net::IpAddr, std::time::Instant>,
    max_requests: usize,
    limit_seconds: u32,
    block_duration_seconds: u32,
}
impl DependcyInjectableService for RateLimitMiddleware {
    fn inject(_service_scope: &crate::services::service_provider::service_provider_scope::ServiceProviderScope) -> Self {
        let config_service = _service_scope.get_service::<ConfigurationService>();
        let config = config_service.get::<RateLimitConfiguration>().unwrap_or_default();
        RateLimitMiddleware {
            ip_request_counts: DashMap::new(),
            ip_blocked_until: DashMap::new(),
            max_requests: config.max_requests,
            limit_seconds: config.limit_seconds,
            block_duration_seconds: config.block_duration_seconds,
        }
    }
}
#[async_trait::async_trait]
impl Middleware for RateLimitMiddleware {
    async fn invoke_async(&self, context: &mut HttpContext, next: MiddlewareNext) {
        // TODO: impl rate limiting based on IP address and request count within a time window with MemoryCacheService
        let client_ip = context.request.client_addr;
        let now = std::time::Instant::now();
        if let Some(blocked_time) = self.ip_blocked_until.get(&client_ip.ip()) {
            if now < *blocked_time {
                context.response.status_code = http::StatusCode::TOO_MANY_REQUESTS; // Too Many Requests
                context.response.body = http::StatusCode::TOO_MANY_REQUESTS.canonical_reason().unwrap_or("Too Many Requests").as_bytes().to_vec();
                return;
            } else {
                self.ip_blocked_until.remove(&client_ip.ip());
            }
        }
        if let Some(mut history_access) = self.ip_request_counts.get_mut(&client_ip.ip()) {
            history_access.push(now);
            let mut index: usize = 0;
            for i in (0..history_access.len()).rev() {
                if now.duration_since(history_access[i]) > std::time::Duration::from_secs(self.limit_seconds.into()) {
                    index = i;
                    break;
                }
            }
            if history_access.len() - index - 1 > self.max_requests {
                self.ip_blocked_until.insert(client_ip.ip(), std::time::Instant::now() + std::time::Duration::from_secs(self.block_duration_seconds.into()));
                history_access.clear(); // Clear history to start fresh after blocking
            }
            else{
                history_access.drain(..index); // Remove old timestamps outside the time window
            }
        };

        next(context).await;
    }
}

impl Application {
    /// Limit the number of requests from a single IP address within a specified time window.
    pub fn use_rate_limit(&mut self) -> &mut Self {
        self.add_middleware::<RateLimitMiddleware>();
        self
    }
}
