use crate::controller::ActionResult;


#[allow(dead_code)]
pub trait PresetResponse {
    ///200
    fn ok(body: Vec<u8>) -> impl ActionResult;
    ///201
    fn created(body: Vec<u8>) -> impl ActionResult;
    ///204
    fn no_content() -> impl ActionResult;
    ///301
    fn moved_permanently(location: &str) -> impl ActionResult;
    ///302
    fn found(location: &str) -> impl ActionResult;
    ///304
    fn not_modified() -> impl ActionResult;
    ///307
    fn temporary_redirect(location: &str) -> impl ActionResult;
    ///400
    fn bad_request() -> impl ActionResult;
    ///401
    fn unauthorized() -> impl ActionResult;
    ///403
    fn forbidden() -> impl ActionResult;
    ///404
    fn not_found() -> impl ActionResult;
    ///405
    fn method_not_allowed() -> impl ActionResult;
    ///406
    fn not_acceptable() -> impl ActionResult;
    ///413
    fn payload_too_large() -> impl ActionResult;
    ///414
    fn uri_too_long() -> impl ActionResult;
    ///415
    fn unsupported_media_type() -> impl ActionResult;
    ///429
    fn rate_limit_exceeded() -> impl ActionResult;
    ///500
    fn internal_server_error() -> impl ActionResult;
    ///501
    fn not_implemented() -> impl ActionResult;
    ///503
    fn service_unavailable() -> impl ActionResult;
    ///504
    fn gateway_timeout() -> impl ActionResult;
    ///505
    fn http_version_not_supported() -> impl ActionResult;
}
