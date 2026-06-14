use crate::{ApplicationBuilder, services::app_queue::AppQueueService};

impl ApplicationBuilder {
    pub fn add_app_queue(&mut self) -> &mut Self {
        self.add_singleton::<AppQueueService>();
        self
    }
}
