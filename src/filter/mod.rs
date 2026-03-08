use crate::har::EntrySummary;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusGroup {
    Informational,
    Success,
    Redirect,
    ClientError,
    ServerError,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct FilterQuery {
    pub text: String,
    pub method: Option<String>,
    pub status_group: Option<StatusGroup>,
    pub mime_category: Option<String>,
    pub duration_min: Option<f64>,
    pub duration_max: Option<f64>,
}

impl FilterQuery {
    pub fn matches(&self, summary: &EntrySummary) -> bool {
        if let Some(method) = self.method.as_deref() {
            if !method.is_empty() && !summary.method.eq_ignore_ascii_case(method) {
                return false;
            }
        }

        if let Some(group) = self.status_group {
            if !group.matches(summary.status) {
                return false;
            }
        }

        if let Some(category) = self.mime_category.as_deref() {
            if !category.is_empty() {
                let mime_head = summary
                    .mime
                    .split('/')
                    .next()
                    .unwrap_or_default()
                    .trim()
                    .to_ascii_lowercase();
                if mime_head != category.to_ascii_lowercase() {
                    return false;
                }
            }
        }

        if let Some(min) = self.duration_min {
            if summary.duration_ms < min {
                return false;
            }
        }

        if let Some(max) = self.duration_max {
            if summary.duration_ms > max {
                return false;
            }
        }

        let text = self.text.trim().to_ascii_lowercase();
        if text.is_empty() {
            return true;
        }

        let status = summary.status.to_string();
        let haystack = [
            summary.method.to_ascii_lowercase(),
            summary.host.to_ascii_lowercase(),
            summary.path.to_ascii_lowercase(),
            summary.mime.to_ascii_lowercase(),
            status,
        ]
        .join(" ");

        haystack.contains(&text)
    }
}

impl StatusGroup {
    pub fn matches(self, status: u16) -> bool {
        match self {
            StatusGroup::Informational => (100..200).contains(&status),
            StatusGroup::Success => (200..300).contains(&status),
            StatusGroup::Redirect => (300..400).contains(&status),
            StatusGroup::ClientError => (400..500).contains(&status),
            StatusGroup::ServerError => (500..600).contains(&status),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{FilterQuery, StatusGroup};
    use crate::har::EntrySummary;

    fn sample(status: u16, method: &str, mime: &str, duration_ms: f64) -> EntrySummary {
        EntrySummary {
            id: 1,
            started_at: "2025-01-01T00:00:00Z".to_string(),
            method: method.to_string(),
            host: "api.example.com".to_string(),
            path: "/v1/users".to_string(),
            status,
            mime: mime.to_string(),
            req_bytes: 10,
            res_bytes: 20,
            duration_ms,
        }
    }

    #[test]
    fn combines_text_and_status_filter() {
        let summary = sample(404, "GET", "application/json", 230.0);
        let query = FilterQuery {
            text: "users".to_string(),
            status_group: Some(StatusGroup::ClientError),
            ..FilterQuery::default()
        };
        assert!(query.matches(&summary));
    }

    #[test]
    fn rejects_out_of_bounds_duration() {
        let summary = sample(200, "POST", "application/json", 25.0);
        let query = FilterQuery {
            duration_min: Some(30.0),
            ..FilterQuery::default()
        };
        assert!(!query.matches(&summary));
    }

    #[test]
    fn empty_query_matches() {
        let summary = sample(200, "GET", "text/html", 10.0);
        assert!(FilterQuery::default().matches(&summary));
    }
}
