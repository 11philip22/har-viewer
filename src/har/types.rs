use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntryRange {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EntrySummary {
    pub id: usize,
    pub started_at: String,
    pub method: String,
    pub host: String,
    pub path: String,
    pub status: u16,
    pub mime: String,
    pub res_bytes: u64,
    pub duration_ms: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EntryDetail {
    pub request_method: String,
    pub request_path: String,
    pub request_http_version: String,
    pub url: String,
    pub request_headers: Vec<(String, String)>,
    pub request_body: String,
    pub response_http_version: String,
    pub response_status: u16,
    pub response_reason: String,
    pub response_headers: Vec<(String, String)>,
    pub response_body: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IndexResult {
    pub summaries: Vec<EntrySummary>,
    pub ranges: Vec<EntryRange>,
}

#[derive(Debug, Error)]
pub enum HarError {
    #[error("HAR file is not valid UTF-8")]
    InvalidUtf8,
    #[error("HAR JSON is missing log.entries array")]
    MissingEntries,
    #[error("HAR JSON structure is invalid around log.entries")]
    InvalidEntriesShape,
    #[error("Entry range is out of bounds")]
    InvalidRange,
    #[error("Failed to parse JSON: {0}")]
    Json(#[from] serde_json::Error),
}
