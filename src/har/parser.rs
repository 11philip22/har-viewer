use serde::Deserialize;

use super::types::{EntryDetail, EntrySummary, HarError, TimingBreakdown};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct HarEntry {
    #[serde(default)]
    started_date_time: String,
    #[serde(default)]
    time: f64,
    request: HarRequest,
    response: HarResponse,
    #[serde(default)]
    server_ip_address: Option<String>,
    #[serde(default)]
    connection: Option<String>,
    #[serde(default)]
    timings: HarTimings,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct HarRequest {
    method: String,
    url: String,
    #[serde(default)]
    headers: Vec<HarHeader>,
    #[serde(default)]
    post_data: Option<HarPostData>,
    #[serde(default)]
    headers_size: Option<i64>,
    #[serde(default)]
    body_size: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct HarResponse {
    #[serde(default)]
    status: u16,
    #[serde(default)]
    status_text: String,
    #[serde(default)]
    headers: Vec<HarHeader>,
    #[serde(default)]
    content: Option<HarContent>,
    #[serde(default)]
    headers_size: Option<i64>,
    #[serde(default)]
    body_size: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct HarHeader {
    name: String,
    value: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct HarPostData {
    #[serde(default)]
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct HarContent {
    #[serde(default)]
    mime_type: Option<String>,
    #[serde(default)]
    size: Option<i64>,
    #[serde(default)]
    text: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct HarTimings {
    #[serde(default)]
    blocked: Option<f64>,
    #[serde(default)]
    dns: Option<f64>,
    #[serde(default)]
    connect: Option<f64>,
    #[serde(default)]
    ssl: Option<f64>,
    #[serde(default)]
    send: Option<f64>,
    #[serde(default)]
    wait: Option<f64>,
    #[serde(default)]
    receive: Option<f64>,
}

pub fn parse_summary(id: usize, entry_slice: &[u8]) -> Result<EntrySummary, HarError> {
    let entry: HarEntry = serde_json::from_slice(entry_slice)?;

    let (host, path) = split_url(&entry.request.url);
    let mime = entry
        .response
        .content
        .as_ref()
        .and_then(|c| c.mime_type.clone())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "-".to_string());

    let req_bytes = clamp_size(entry.request.headers_size) + clamp_size(entry.request.body_size);

    let response_content_size = entry.response.content.as_ref().and_then(|c| c.size);
    let res_body_size = clamp_size(entry.response.body_size).max(clamp_size(response_content_size));
    let res_bytes = clamp_size(entry.response.headers_size) + res_body_size;

    let duration_ms = if entry.time >= 0.0 {
        entry.time
    } else {
        timings_to_duration(&entry.timings)
    };

    Ok(EntrySummary {
        id,
        started_at: entry.started_date_time,
        method: entry.request.method,
        host,
        path,
        status: entry.response.status,
        mime,
        req_bytes,
        res_bytes,
        duration_ms,
    })
}

pub fn parse_detail(entry_slice: &[u8]) -> Result<EntryDetail, HarError> {
    let entry: HarEntry = serde_json::from_slice(entry_slice)?;

    let (_, path) = split_url(&entry.request.url);
    let request_line = format!("{} {}", entry.request.method, path);

    let request_headers = entry
        .request
        .headers
        .into_iter()
        .map(|h| (h.name, h.value))
        .collect();

    let response_headers = entry
        .response
        .headers
        .into_iter()
        .map(|h| (h.name, h.value))
        .collect();

    let request_body = entry
        .request
        .post_data
        .and_then(|pd| pd.text)
        .unwrap_or_default();

    let response_body = entry
        .response
        .content
        .as_ref()
        .and_then(|content| content.text.clone())
        .unwrap_or_default();

    let timings = TimingBreakdown {
        blocked: clamp_duration(entry.timings.blocked),
        dns: clamp_duration(entry.timings.dns),
        connect: clamp_duration(entry.timings.connect),
        ssl: clamp_duration(entry.timings.ssl),
        send: clamp_duration(entry.timings.send),
        wait: clamp_duration(entry.timings.wait),
        receive: clamp_duration(entry.timings.receive),
    };

    Ok(EntryDetail {
        request_line,
        url: entry.request.url,
        request_headers,
        request_body,
        response_status: entry.response.status,
        response_reason: entry.response.status_text,
        response_headers,
        response_body,
        timings,
        server_ip: entry.server_ip_address,
        connection: entry.connection,
    })
}

fn split_url(raw: &str) -> (String, String) {
    match url::Url::parse(raw) {
        Ok(url) => {
            let host = url.host_str().unwrap_or("-").to_string();
            let mut path = url.path().to_string();
            if let Some(query) = url.query() {
                path.push('?');
                path.push_str(query);
            }
            (host, path)
        }
        Err(_) => ("-".to_string(), raw.to_string()),
    }
}

fn clamp_size(size: Option<i64>) -> u64 {
    size.unwrap_or(0).max(0) as u64
}

fn clamp_duration(duration: Option<f64>) -> f64 {
    duration.unwrap_or(0.0).max(0.0)
}

fn timings_to_duration(timings: &HarTimings) -> f64 {
    clamp_duration(timings.blocked)
        + clamp_duration(timings.dns)
        + clamp_duration(timings.connect)
        + clamp_duration(timings.ssl)
        + clamp_duration(timings.send)
        + clamp_duration(timings.wait)
        + clamp_duration(timings.receive)
}

#[cfg(test)]
mod tests {
    use super::{parse_detail, parse_summary};

    #[test]
    fn parses_summary_with_optional_fields() {
        let entry = br#"{
          "startedDateTime":"2025-01-01T00:00:00.000Z",
          "time":123.4,
          "request":{
            "method":"GET",
            "url":"https://example.com/api?q=1",
            "headers":[],
            "headersSize":100,
            "bodySize":-1
          },
          "response":{
            "status":200,
            "statusText":"OK",
            "headers":[],
            "content":{"mimeType":"application/json","size":42,"text":"{\"ok\":true}"},
            "headersSize":150,
            "bodySize":40
          },
          "timings":{"wait":10}
        }"#;

        let summary = parse_summary(7, entry).expect("summary");
        assert_eq!(summary.id, 7);
        assert_eq!(summary.method, "GET");
        assert_eq!(summary.host, "example.com");
        assert_eq!(summary.path, "/api?q=1");
        assert_eq!(summary.status, 200);
        assert_eq!(summary.mime, "application/json");
        assert_eq!(summary.req_bytes, 100);
        assert_eq!(summary.res_bytes, 192);
        assert_eq!(summary.duration_ms, 123.4);
    }

    #[test]
    fn parses_detail_with_full_bodies() {
        let entry = br#"{
          "startedDateTime":"2025-01-01T00:00:00.000Z",
          "time":10,
          "request":{
            "method":"POST",
            "url":"https://example.com/login",
            "headers":[{"name":"content-type","value":"application/json"}],
            "postData":{"text":"{\"user\":\"alice\"}"}
          },
          "response":{
            "status":401,
            "statusText":"Unauthorized",
            "headers":[{"name":"content-type","value":"application/json"}],
            "content":{"mimeType":"application/json","text":"{\"error\":\"bad creds\"}"}
          },
          "timings":{"send":1,"wait":8,"receive":1}
        }"#;

        let detail = parse_detail(entry).expect("detail");
        assert_eq!(detail.request_line, "POST /login");
        assert_eq!(detail.response_status, 401);
        assert!(detail.request_body.contains("alice"));
        assert!(detail.response_body.contains("error"));
        assert_eq!(detail.timings.wait, 8.0);
    }
}
