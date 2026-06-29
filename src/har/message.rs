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
