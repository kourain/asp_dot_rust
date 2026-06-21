use asp_dot_rust::{
    api_controller,
    controller::{ActionResult, get, post, put, route},
    controller_route,
    logging::LOGGER,
};

api_controller!(pub HomeController {
    temp: String,
    temp2: String,
});

#[controller_route("")]
impl HomeController {
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

    #[route(["GET", "POST"], "/health")]
    pub async fn health(&self) -> impl ActionResult {
        "ok".to_string()
    }
}
