mod message;
mod parser;
mod scanner;
mod types;

#[cfg(target_arch = "wasm32")]
pub(crate) use message::{build_request_message, build_response_message};
pub use types::{EntryDetail, EntryRange, EntrySummary, HarError, IndexResult};

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

    for (id, range) in ranges.iter().enumerate() {
        let entry_slice = file_bytes
            .get(range.start..range.end)
            .ok_or(HarError::InvalidRange)?;
        summaries.push(parser::parse_summary(id, entry_slice)?);

        if id % 128 == 0 {
            on_progress(id + 1, total);
            gloo_timers::future::TimeoutFuture::new(0).await;
        }
    }

    on_progress(total, total);

    Ok(IndexResult { summaries, ranges })
}

pub fn load_detail(file_bytes: &[u8], range: EntryRange) -> Result<EntryDetail, HarError> {
    let entry_slice = file_bytes
        .get(range.start..range.end)
        .ok_or(HarError::InvalidRange)?;
    parser::parse_detail(entry_slice)
}
