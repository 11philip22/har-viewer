use crate::har::EntrySummary;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct FilterQuery {
    pub text: String,
    pub method: Option<String>,
    pub status_class: Option<u16>,
    pub mime_category: Option<String>,
}

impl FilterQuery {
    pub fn matches(&self, summary: &EntrySummary) -> bool {
        if let Some(method) = self.method.as_deref() {
            if !method.is_empty() && !summary.method.eq_ignore_ascii_case(method) {
                return false;
            }
        }

        if let Some(class) = self.status_class {
            if summary.status / 100 != class {
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
