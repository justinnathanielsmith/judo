use chrono::{DateTime, Local};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ErrorSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ErrorState {
    pub message: String,
    pub timestamp: DateTime<Local>,
    pub severity: ErrorSeverity,
    pub suggestions: Vec<String>,
}
