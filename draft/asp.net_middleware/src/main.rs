use tokio;

use crate::{
    code::app::HttpContext,
    demo_mw::{CountNoDIMiddleware, CountRequestDIMiddleware, CountStartupDIMiddleware},
};

mod code;
mod demo_mw;
#[tokio::main]
async fn main() {
    let mut app = code::app::Application::new();
    app.add_middleware::<CountNoDIMiddleware>();
    app.add_middleware::<CountRequestDIMiddleware>();
    app.add_middleware::<CountStartupDIMiddleware>();
    let mut ctx = HttpContext::default();
    ctx.http_request = "requested\n".into();
    app.run(&mut ctx).await;
    println!("END");
}
