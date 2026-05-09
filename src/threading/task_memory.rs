tokio::task_local! {
   pub(crate) static HTTP_REQUEST_ID: String;
   pub(crate) static TCP_CONNECTION_ID: String;
}
