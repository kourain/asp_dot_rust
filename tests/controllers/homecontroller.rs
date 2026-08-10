use asp_dot_rust::{
    api_controller,
    controller::{ActionResult, get, post, put, route},
    http_context::HttpContextRef,
    logging::LOGGER,
};
use asp_dot_rust_macros::{controller_inject_require, controller_route};

api_controller!(pub HomeController {
    temp: String,
});

#[controller_route("")]
#[controller_inject_require]
impl HomeController {
    fn new(http_context: HttpContextRef) -> Self {
        Self {
            temp: Default::default(),
            http_context,
        }
    }
    #[get("/")]
    pub async fn index(&mut self) -> impl ActionResult {
        LOGGER::info("Handling index action".to_string());
        self.http_context.response.headers.insert_str("Content-Type", "text/html");
        self.http_context.response.status_code = http::StatusCode::OK;
        "<html><body><h1>Hello, World!</h1></body></html>"
    }

    #[post("/update")]
    pub fn update(&mut self) -> impl ActionResult {
        self.temp.clone()
    }

    #[put("/replace")]
    pub async fn replace(&mut self) -> impl ActionResult {
        self.temp.clone()
    }

    #[route(["get", "post"], "/health")]
    pub async fn health(&self) -> impl ActionResult {
        "ok".to_string()
    }
}
