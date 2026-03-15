use crate::har::EntryDetail;

pub fn build_request_message(detail: &EntryDetail) -> String {
    let mut lines: Vec<String> = Vec::new();
    lines.push(format!(
        "{} {} {}",
        detail.request_method,
        display_request_path(&detail.request_path),
        display_http_version(&detail.request_http_version)
    ));

    let has_host = detail
        .request_headers
        .iter()
        .any(|(name, _)| name.eq_ignore_ascii_case("host"));

    if !has_host {
        if let Some(host) = derive_host(&detail.url) {
            lines.push(format!("Host: {host}"));
        }
    }

    for (name, value) in &detail.request_headers {
        lines.push(format!("{name}: {value}"));
    }

    let mut message = lines.join("\n");
    message.push_str("\n\n");
    message.push_str(&pretty_json_if_possible(&detail.request_body));
    message
}

pub fn build_response_message(detail: &EntryDetail) -> String {
    let mut lines: Vec<String> = Vec::new();
    let version = display_http_version(&detail.response_http_version);
    let reason = detail.response_reason.trim();

    if reason.is_empty() {
        lines.push(format!("{} {}", version, detail.response_status));
    } else {
        lines.push(format!("{} {} {}", version, detail.response_status, reason));
    }

    for (name, value) in &detail.response_headers {
        lines.push(format!("{name}: {value}"));
    }

    let mut message = lines.join("\n");
    message.push_str("\n\n");
    message.push_str(&pretty_json_if_possible(&detail.response_body));
    message
}

pub fn pretty_json_if_possible(body: &str) -> String {
    if body.trim().is_empty() {
        return String::new();
    }

    match serde_json::from_str::<serde_json::Value>(body) {
        Ok(value) => serde_json::to_string_pretty(&value).unwrap_or_else(|_| body.to_string()),
        Err(_) => body.to_string(),
    }
}

fn display_request_path(path: &str) -> &str {
    if path.trim().is_empty() { "/" } else { path }
}

fn display_http_version(version: &str) -> &str {
    let trimmed = version.trim();
    if trimmed.is_empty() {
        "HTTP/1.1"
    } else {
        trimmed
    }
}

fn derive_host(url: &str) -> Option<String> {
    match url::Url::parse(url) {
        Ok(parsed) => parsed.host_str().map(|value| value.to_string()),
        Err(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use crate::har::{EntryDetail, TimingBreakdown};

    use super::{build_request_message, build_response_message, pretty_json_if_possible};

    fn sample_detail() -> EntryDetail {
        EntryDetail {
            request_method: "POST".to_string(),
            request_path: "/api/albums".to_string(),
            request_http_version: "HTTP/2".to_string(),
            url: "https://cyberdrop.cr/api/albums".to_string(),
            request_headers: vec![
                ("Content-Type".to_string(), "application/json".to_string()),
                ("Token".to_string(), "abc123".to_string()),
            ],
            request_body: "{\"name\":\"test112\",\"description\":\"\"}".to_string(),
            response_http_version: "HTTP/2".to_string(),
            response_status: 200,
            response_reason: "OK".to_string(),
            response_headers: vec![(
                "Content-Type".to_string(),
                "application/json; charset=utf-8".to_string(),
            )],
            response_body: "{\"success\":true,\"id\":146047}".to_string(),
            timings: TimingBreakdown::default(),
            server_ip: None,
            connection: None,
        }
    }

    #[test]
    fn pretty_prints_valid_json() {
        let pretty = pretty_json_if_possible("{\"a\":1,\"b\":2}");
        assert!(pretty.contains("\n"));
        assert!(pretty.contains("\"a\": 1"));
    }

    #[test]
    fn keeps_invalid_json_unchanged() {
        let raw = "{this is not json";
        assert_eq!(pretty_json_if_possible(raw), raw);
    }

    #[test]
    fn request_message_adds_host_fallback_and_pretty_body() {
        let detail = sample_detail();
        let message = build_request_message(&detail);

        assert!(message.starts_with("POST /api/albums HTTP/2"));
        assert!(message.contains("\nHost: cyberdrop.cr\n"));
        assert!(message.contains("\n\n{\n"));
    }

    #[test]
    fn response_message_uses_start_line_and_pretty_body() {
        let detail = sample_detail();
        let message = build_response_message(&detail);

        assert!(message.starts_with("HTTP/2 200 OK"));
        assert!(message.contains("Content-Type: application/json; charset=utf-8"));
        assert!(message.contains("\n\n{\n"));
    }

    #[test]
    fn request_message_keeps_existing_host_header_order() {
        let mut detail = sample_detail();
        detail
            .request_headers
            .insert(0, ("Host".to_string(), "manual.host".to_string()));

        let message = build_request_message(&detail);
        let host_count = message.matches("Host:").count();

        assert_eq!(host_count, 1);
        assert!(message.contains("\nHost: manual.host\n"));
    }
}
