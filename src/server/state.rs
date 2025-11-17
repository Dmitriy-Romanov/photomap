use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;
use crate::database::Database;
use crate::settings::Settings;
use super::events::ProcessingEvent;

// Application state for sharing database and settings
#[derive(Clone)]
pub struct AppState {
    pub db: Database,
    pub settings: Arc<Mutex<Settings>>,
    pub event_sender: broadcast::Sender<ProcessingEvent>,
}