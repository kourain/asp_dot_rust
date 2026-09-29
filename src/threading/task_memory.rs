use std::sync::Arc;

tokio::task_local! {
   pub(crate) static HTTP_REQUEST_ID: Arc<str>;
   pub(crate) static TCP_CONNECTION_ID: Arc<str>;
}

pub(crate) fn get_http_request_id() -> Arc<str> {
    crate::threading::HTTP_REQUEST_ID.try_with(|id| id.clone()).unwrap_or_default()
}
pub(crate) fn get_connection_id() -> Arc<str> {
    crate::threading::TCP_CONNECTION_ID.try_with(|id| id.clone()).unwrap_or_default()
}
