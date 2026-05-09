#[async_trait::async_trait]
pub trait Routing: 'static + Sized
where
    Self: Sized,
{
    async fn routing(&mut self, method_name: String);
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionRoute {
    pub action_name: &'static str,
    pub method: Vec<&'static str>,
    pub route: &'static str,
}

impl ActionRoute {
    pub const fn new(action_name: &'static str, method: Vec<&'static str>, route: &'static str) -> Self {
        Self { action_name, method, route }
    }
}
