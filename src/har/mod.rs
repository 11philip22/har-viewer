mod parser;
mod scanner;
mod types;

pub use types::{
    EntryDetail, EntryRange, EntrySummary, HarError, IndexResult, IndexStats, TimingBreakdown,
};

pub struct HarIndexer;

impl HarIndexer {
    pub fn index(file_bytes: &[u8]) -> Result<IndexResult, HarError> {
        let ranges = scanner::scan_entry_ranges(file_bytes)?;
        let mut summaries = Vec::with_capacity(ranges.len());
        let mut indexed_bytes = 0usize;

        for (id, range) in ranges.iter().enumerate() {
            let entry_slice = file_bytes
                .get(range.start..range.end)
                .ok_or(HarError::InvalidRange)?;
            summaries.push(parser::parse_summary(id, entry_slice)?);
            indexed_bytes = indexed_bytes.saturating_add(range.len());
        }

        Ok(IndexResult {
            stats: IndexStats {
                entry_count: summaries.len(),
                indexed_bytes,
            },
            summaries,
            ranges,
        })
    }

    #[cfg(target_arch = "wasm32")]
    pub async fn index_cooperative<F>(
        file_bytes: &[u8],
        mut on_progress: F,
    ) -> Result<IndexResult, HarError>
    where
        F: FnMut(usize, usize),
    {
        let ranges = scanner::scan_entry_ranges(file_bytes)?;
        let total = ranges.len();
        let mut summaries = Vec::with_capacity(total);
        let mut indexed_bytes = 0usize;

        for (id, range) in ranges.iter().enumerate() {
            let entry_slice = file_bytes
                .get(range.start..range.end)
                .ok_or(HarError::InvalidRange)?;
            summaries.push(parser::parse_summary(id, entry_slice)?);
            indexed_bytes = indexed_bytes.saturating_add(range.len());

            if id % 128 == 0 {
                on_progress(id + 1, total);
                gloo_timers::future::TimeoutFuture::new(0).await;
            }
        }

        on_progress(total, total);

        Ok(IndexResult {
            stats: IndexStats {
                entry_count: summaries.len(),
                indexed_bytes,
            },
            summaries,
            ranges,
        })
    }

    pub fn load_detail(file_bytes: &[u8], range: EntryRange) -> Result<EntryDetail, HarError> {
        let entry_slice = file_bytes
            .get(range.start..range.end)
            .ok_or(HarError::InvalidRange)?;
        parser::parse_detail(entry_slice)
    }
}

#[cfg(test)]
mod tests {
    use super::HarIndexer;

    fn sample_har(entries: &str) -> Vec<u8> {
        format!("{{\"log\":{{\"entries\":[{}]}}}}", entries).into_bytes()
    }

    #[test]
    fn load_detail_uses_selected_range() {
        let har = sample_har(
            r#"{"startedDateTime":"2025-01-01T00:00:00.000Z","time":1,"request":{"method":"GET","url":"https://a.test/","headers":[]},"response":{"status":200,"statusText":"OK","headers":[],"content":{"text":"A"}},"timings":{}}
               ,{"startedDateTime":"2025-01-01T00:00:01.000Z","time":1,"request":{"method":"GET","url":"https://b.test/","headers":[]},"response":{"status":500,"statusText":"ERR","headers":[],"content":{"text":"B"}},"timings":{}}"#,
        );

        let result = HarIndexer::index(&har).expect("index");
        let detail = HarIndexer::load_detail(&har, result.ranges[1]).expect("detail");

        assert_eq!(detail.response_status, 500);
        assert_eq!(detail.response_body, "B");
    }

    #[test]
    fn performance_guard_large_synthetic_payload() {
        let mut entries = String::new();
        for i in 0..5_000u32 {
            if i > 0 {
                entries.push(',');
            }
            entries.push_str(&format!(
                "{{\"startedDateTime\":\"2025-01-01T00:00:{:02}.000Z\",\"time\":5,\"request\":{{\"method\":\"GET\",\"url\":\"https://example.test/item/{}\",\"headers\":[]}},\"response\":{{\"status\":200,\"statusText\":\"OK\",\"headers\":[],\"content\":{{\"mimeType\":\"text/plain\",\"text\":\"x\"}}}},\"timings\":{{\"wait\":5}}}}",
                i % 60,
                i
            ));
        }

        let har = sample_har(&entries);
        let result = HarIndexer::index(&har).expect("index synthetic");
        assert_eq!(result.stats.entry_count, 5_000);
    }
}
