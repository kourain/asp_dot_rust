// use asp_dot_rust_macros::middleware;

// use crate::{
//     Application, MutexAsync,
//     configuration::RateLimitConfiguration,
//     http_context::{self, HttpContext},
//     middleware::{Middleware, MiddlewareNext},
// };
// #[derive(Default)]
// pub struct RateLimitMiddleware {
//     ip_request_counts: MutexAsync<std::collections::HashMap<std::net::IpAddr, (usize, Vec<std::time::Instant>)>>,
//     ip_blocked_until: MutexAsync<std::collections::HashMap<std::net::IpAddr, std::time::Instant>>,
//     max_requests: usize,
//     limit_seconds: u32,
//     block_duration_seconds: u32,
// }
// #[middleware]
// impl Middleware for RateLimitMiddleware {
//     fn with_application(&mut self, app: &crate::application::Application) {
//         let config = app.get_configuration::<RateLimitConfiguration>();
//         self.max_requests = config.max_requests;
//         self.limit_seconds = config.limit_seconds;
//         self.block_duration_seconds = config.block_duration_seconds;
//     }
//     async fn invoke_async<'a>(&self, context: &'a mut HttpContext, next: MiddlewareNext) {
//         let client_ip = http_context.request.client_addr;
//         let now = std::time::Instant::now();
//         let mut blocked_until = this.ip_blocked_until.lock().await;
//         {
//             if let Some(blocked_time) = blocked_until.get(&client_ip) {
//                 if now < *blocked_time {
//                     context.response.status_code = http::StatusCode::TOO_MANY_REQUESTS; // Too Many Requests
//                     context.response.body = http::StatusCode::TOO_MANY_REQUESTS.canonical_reason().unwrap_or("Too Many Requests").as_bytes().to_vec();
//                     return;
//                 } else {
//                     blocked_until.remove(&client_ip);
//                 }
//             }
//         }

//         let should_block = {
//             let mut counts = self.ip_request_counts.lock().await;
//             let (count, history_access) = counts.entry(client_ip).or_insert((0, Vec::new()));
//             *count += 1;
//             history_access.push(now);

//             let should_block = *count > self.max_requests && history_access.iter().filter(|&&t| t.elapsed() < std::time::Duration::from_secs(self.limit_seconds.into())).count() >= self.max_requests;

//             if should_block {
//                 history_access.clear(); // Clear history to start fresh after blocking
//             }
//             should_block
//         };

//         if should_block {
//             let mut blocked_until = self.ip_blocked_until.lock().await;
//             blocked_until.insert(client_ip, std::time::Instant::now() + std::time::Duration::from_secs(self.block_duration_seconds.into()));
//             return;
//         }

//         next(context).await;
//     }
// }

// impl Application {
//     /// Limit the number of requests from a single IP address within a specified time window.
//     pub fn use_rate_limit(&mut self) -> &mut Self {
//         self.add_middleware::<RateLimitMiddleware>();
//         self
//     }
// }
