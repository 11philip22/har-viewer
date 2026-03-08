use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntryRange {
    pub start: usize,
    pub end: usize,
}

impl EntryRange {
    pub fn len(self) -> usize {
        self.end.saturating_sub(self.start)
    }

    pub fn is_empty(self) -> bool {
        self.len() == 0
    }
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
    pub req_bytes: u64,
    pub res_bytes: u64,
    pub duration_ms: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct TimingBreakdown {
    pub blocked: f64,
    pub dns: f64,
    pub connect: f64,
    pub ssl: f64,
    pub send: f64,
    pub wait: f64,
    pub receive: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EntryDetail {
    pub request_line: String,
    pub url: String,
    pub request_headers: Vec<(String, String)>,
    pub request_body: String,
    pub response_status: u16,
    pub response_reason: String,
    pub response_headers: Vec<(String, String)>,
    pub response_body: String,
    pub timings: TimingBreakdown,
    pub server_ip: Option<String>,
    pub connection: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct IndexStats {
    pub entry_count: usize,
    pub indexed_bytes: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IndexResult {
    pub summaries: Vec<EntrySummary>,
    pub ranges: Vec<EntryRange>,
    pub stats: IndexStats,
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
