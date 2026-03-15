use serde::{Deserialize, Deserializer};

use super::types::{EntryDetail, EntrySummary, HarError, TimingBreakdown};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct HarEntry {
    #[serde(default)]
    started_date_time: String,
    #[serde(default, deserialize_with = "de_f64_flexible")]
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
    http_version: Option<String>,
    #[serde(default)]
    headers: Vec<HarHeader>,
    #[serde(default)]
    post_data: Option<HarPostData>,
    #[serde(default, deserialize_with = "de_opt_i64_flexible")]
    headers_size: Option<i64>,
    #[serde(default, deserialize_with = "de_opt_i64_flexible")]
    body_size: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct HarResponse {
    #[serde(default, deserialize_with = "de_u16_flexible")]
    status: u16,
    #[serde(default)]
    status_text: String,
    #[serde(default)]
    http_version: Option<String>,
    #[serde(default)]
    headers: Vec<HarHeader>,
    #[serde(default)]
    content: Option<HarContent>,
    #[serde(default, deserialize_with = "de_opt_i64_flexible")]
    headers_size: Option<i64>,
    #[serde(default, deserialize_with = "de_opt_i64_flexible")]
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
    #[serde(default, deserialize_with = "de_opt_i64_flexible")]
    size: Option<i64>,
    #[serde(default)]
    text: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct HarTimings {
    #[serde(default, deserialize_with = "de_opt_f64_flexible")]
    blocked: Option<f64>,
    #[serde(default, deserialize_with = "de_opt_f64_flexible")]
    dns: Option<f64>,
    #[serde(default, deserialize_with = "de_opt_f64_flexible")]
    connect: Option<f64>,
    #[serde(default, deserialize_with = "de_opt_f64_flexible")]
    ssl: Option<f64>,
    #[serde(default, deserialize_with = "de_opt_f64_flexible")]
    send: Option<f64>,
    #[serde(default, deserialize_with = "de_opt_f64_flexible")]
    wait: Option<f64>,
    #[serde(default, deserialize_with = "de_opt_f64_flexible")]
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
        request_method: entry.request.method,
        request_path: if path.is_empty() {
            "/".to_string()
        } else {
            path
        },
        request_http_version: normalize_http_version(entry.request.http_version.as_deref()),
        url: entry.request.url,
        request_headers,
        request_body,
        response_http_version: normalize_http_version(entry.response.http_version.as_deref()),
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

fn normalize_http_version(version: Option<&str>) -> String {
    let normalized = version
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_ascii_uppercase())
        .unwrap_or_else(|| "HTTP/1.1".to_string());

    if normalized.starts_with("HTTP/") {
        normalized
    } else {
        "HTTP/1.1".to_string()
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

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum NumberLike {
    I64(i64),
    F64(f64),
    Str(String),
}

fn de_f64_flexible<'de, D>(deserializer: D) -> Result<f64, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<NumberLike>::deserialize(deserializer)?;
    number_like_to_f64(value, 0.0)
}

fn de_u16_flexible<'de, D>(deserializer: D) -> Result<u16, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<NumberLike>::deserialize(deserializer)?;
    let parsed = number_like_to_i64(value, 0)?;
    Ok(parsed.clamp(0, u16::MAX as i64) as u16)
}

fn de_opt_i64_flexible<'de, D>(deserializer: D) -> Result<Option<i64>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<NumberLike>::deserialize(deserializer)?;
    match value {
        None => Ok(None),
        Some(n) => Ok(Some(number_like_to_i64(Some(n), 0)?)),
    }
}

fn de_opt_f64_flexible<'de, D>(deserializer: D) -> Result<Option<f64>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<NumberLike>::deserialize(deserializer)?;
    match value {
        None => Ok(None),
        Some(n) => Ok(Some(number_like_to_f64(Some(n), 0.0)?)),
    }
}

fn number_like_to_i64<E>(value: Option<NumberLike>, default: i64) -> Result<i64, E>
where
    E: serde::de::Error,
{
    match value {
        None => Ok(default),
        Some(NumberLike::I64(v)) => Ok(v),
        Some(NumberLike::F64(v)) => Ok(v as i64),
        Some(NumberLike::Str(v)) => {
            let trimmed = v.trim();
            if trimmed.is_empty() {
                Ok(default)
            } else if let Ok(parsed) = trimmed.parse::<i64>() {
                Ok(parsed)
            } else if let Ok(parsed) = trimmed.parse::<f64>() {
                Ok(parsed as i64)
            } else {
                Err(E::custom(format!("invalid numeric value: {trimmed}")))
            }
        }
    }
}

fn number_like_to_f64<E>(value: Option<NumberLike>, default: f64) -> Result<f64, E>
where
    E: serde::de::Error,
{
    match value {
        None => Ok(default),
        Some(NumberLike::I64(v)) => Ok(v as f64),
        Some(NumberLike::F64(v)) => Ok(v),
        Some(NumberLike::Str(v)) => {
            let trimmed = v.trim();
            if trimmed.is_empty() {
                Ok(default)
            } else if let Ok(parsed) = trimmed.parse::<f64>() {
                Ok(parsed)
            } else {
                Err(E::custom(format!("invalid numeric value: {trimmed}")))
            }
        }
    }
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
            "httpVersion":"HTTP/2",
            "headers":[{"name":"content-type","value":"application/json"}],
            "postData":{"text":"{\"user\":\"alice\"}"}
          },
          "response":{
            "status":401,
            "statusText":"Unauthorized",
            "httpVersion":"HTTP/2",
            "headers":[{"name":"content-type","value":"application/json"}],
            "content":{"mimeType":"application/json","text":"{\"error\":\"bad creds\"}"}
          },
          "timings":{"send":1,"wait":8,"receive":1}
        }"#;

        let detail = parse_detail(entry).expect("detail");
        assert_eq!(detail.request_method, "POST");
        assert_eq!(detail.request_path, "/login");
        assert_eq!(detail.request_http_version, "HTTP/2");
        assert_eq!(detail.response_http_version, "HTTP/2");
        assert_eq!(detail.response_status, 401);
        assert!(detail.request_body.contains("alice"));
        assert!(detail.response_body.contains("error"));
        assert_eq!(detail.timings.wait, 8.0);
    }

    #[test]
    fn parses_string_encoded_numbers() {
        let entry = br#"{
          "startedDateTime":"2025-01-01T00:00:00.000Z",
          "time":"579",
          "request":{
            "method":"GET",
            "url":"https://example.com/robots.txt",
            "headers":[],
            "headersSize":"-1",
            "bodySize":"23093"
          },
          "response":{
            "status":"200",
            "statusText":"OK",
            "headers":[],
            "content":{"mimeType":"text/plain","size":"23093","text":"hello"},
            "headersSize":"-1",
            "bodySize":"23093"
          },
          "timings":{"send":"0","wait":"579","receive":"0"}
        }"#;

        let summary = parse_summary(0, entry).expect("summary");
        assert_eq!(summary.res_bytes, 23093);
        assert_eq!(summary.req_bytes, 23093);
        assert_eq!(summary.duration_ms, 579.0);

        let detail = parse_detail(entry).expect("detail");
        assert_eq!(detail.response_status, 200);
        assert_eq!(detail.timings.wait, 579.0);
    }

    #[test]
    fn defaults_http_version_when_missing() {
        let entry = br#"{
          "startedDateTime":"2025-01-01T00:00:00.000Z",
          "time":1,
          "request":{
            "method":"GET",
            "url":"https://example.com/",
            "headers":[]
          },
          "response":{
            "status":200,
            "statusText":"OK",
            "headers":[]
          },
          "timings":{}
        }"#;

        let detail = parse_detail(entry).expect("detail");
        assert_eq!(detail.request_http_version, "HTTP/1.1");
        assert_eq!(detail.response_http_version, "HTTP/1.1");
    }
}
