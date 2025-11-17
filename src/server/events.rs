use serde::{Deserialize, Serialize};

// SSE Event types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingEvent {
    pub event_type: String,
    pub data: ProcessingData,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProcessingData {
    pub total_files: Option<usize>,
    pub processed: Option<usize>,
    pub gps_found: Option<usize>,
    pub no_gps: Option<usize>,
    pub heic_files: Option<usize>,
    pub skipped: Option<usize>,
    pub current_file: Option<String>,
    pub speed: Option<f64>,
    pub eta: Option<String>,
    pub message: Option<String>,
    pub phase: Option<String>,
}
