use cimmeria_services::orchestrator::Orchestrator;
use std::sync::Arc;

pub struct AppState {
    pub orchestrator: Arc<Orchestrator>,
}

impl AppState {
    pub fn new(orchestrator: Arc<Orchestrator>) -> Self {
        Self { orchestrator }
    }
}
