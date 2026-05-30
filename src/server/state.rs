use super::events::ProcessingEvent;
use crate::database::Database;
use crate::settings::Settings;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::sync::{broadcast, mpsc};

// Application state for sharing database and settings
#[derive(Clone)]
pub struct AppState {
    pub db: Database,
    pub settings: Arc<Mutex<Settings>>,
    pub event_sender: mpsc::Sender<ProcessingEvent>,
    pub event_broadcast: broadcast::Sender<ProcessingEvent>,
    pub shutdown_sender: broadcast::Sender<()>,
}
