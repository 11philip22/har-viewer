use std::cmp::Ordering;
use std::collections::HashMap;
use std::sync::Arc;

use crate::filter::FilterQuery;
use crate::har::{EntryDetail, EntryRange, EntrySummary, IndexResult, IndexStats};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InspectorTab {
    Request,
    Response,
    Headers,
    Timings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortColumn {
    StartedAt,
    Method,
    Host,
    Path,
    Status,
    Mime,
    ReqBytes,
    ResBytes,
    Duration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortDirection {
    Asc,
    Desc,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SortConfig {
    pub column: SortColumn,
    pub direction: SortDirection,
}

impl Default for SortConfig {
    fn default() -> Self {
        Self {
            column: SortColumn::StartedAt,
            direction: SortDirection::Desc,
        }
    }
}

#[derive(Debug, Clone)]
pub struct HarStore {
    pub file_bytes: Option<Arc<[u8]>>,
    pub entries: Vec<EntrySummary>,
    pub entry_ranges: Vec<EntryRange>,
    pub selected_row: Option<usize>,
    pub active_tab: InspectorTab,
    pub filter: FilterQuery,
    pub sort: SortConfig,
    pub details: HashMap<usize, EntryDetail>,
    pub stats: Option<IndexStats>,
    pub indexing: bool,
    pub indexing_progress: f32,
    pub error: Option<String>,
}

impl Default for HarStore {
    fn default() -> Self {
        Self {
            file_bytes: None,
            entries: Vec::new(),
            entry_ranges: Vec::new(),
            selected_row: None,
            active_tab: InspectorTab::Request,
            filter: FilterQuery::default(),
            sort: SortConfig::default(),
            details: HashMap::new(),
            stats: None,
            indexing: false,
            indexing_progress: 0.0,
            error: None,
        }
    }
}

impl HarStore {
    pub fn clear(&mut self) {
        self.file_bytes = None;
        self.entries.clear();
        self.entry_ranges.clear();
        self.selected_row = None;
        self.active_tab = InspectorTab::Request;
        self.details.clear();
        self.stats = None;
        self.indexing = false;
        self.indexing_progress = 0.0;
        self.error = None;
    }

    pub fn begin_indexing(&mut self) {
        self.clear();
        self.indexing = true;
    }

    pub fn set_index_result(&mut self, bytes: Arc<[u8]>, result: IndexResult) {
        self.file_bytes = Some(bytes);
        self.entries = result.summaries;
        self.entry_ranges = result.ranges;
        self.stats = Some(result.stats);
        self.details.clear();
        self.selected_row = if self.entries.is_empty() {
            None
        } else {
            Some(0)
        };
        self.indexing = false;
        self.indexing_progress = 1.0;
        self.error = None;
    }

    pub fn set_error(&mut self, message: impl Into<String>) {
        self.indexing = false;
        self.error = Some(message.into());
    }

    pub fn visible_indices(&self) -> Vec<usize> {
        let mut visible: Vec<usize> = self
            .entries
            .iter()
            .enumerate()
            .filter_map(|(idx, entry)| self.filter.matches(entry).then_some(idx))
            .collect();

        let sort = self.sort;
        visible.sort_by(|left_idx, right_idx| {
            let left = &self.entries[*left_idx];
            let right = &self.entries[*right_idx];
            let ordering = compare_entries(left, right, sort.column).then(left.id.cmp(&right.id));
            match sort.direction {
                SortDirection::Asc => ordering,
                SortDirection::Desc => ordering.reverse(),
            }
        });

        visible
    }

    pub fn toggle_sort(&mut self, column: SortColumn) {
        if self.sort.column == column {
            self.sort.direction = match self.sort.direction {
                SortDirection::Asc => SortDirection::Desc,
                SortDirection::Desc => SortDirection::Asc,
            };
        } else {
            self.sort.column = column;
            self.sort.direction = SortDirection::Asc;
        }
    }

    pub fn selected_summary(&self) -> Option<&EntrySummary> {
        self.selected_row.and_then(|idx| self.entries.get(idx))
    }

    pub fn selected_detail(&self) -> Option<&EntryDetail> {
        self.selected_row.and_then(|idx| self.details.get(&idx))
    }

    pub fn selected_range(&self) -> Option<EntryRange> {
        self.selected_row
            .and_then(|idx| self.entry_ranges.get(idx).copied())
    }

    pub fn move_selection(&mut self, delta: isize, visible: &[usize]) {
        if visible.is_empty() {
            self.selected_row = None;
            return;
        }

        let current_position = self
            .selected_row
            .and_then(|selected| visible.iter().position(|idx| *idx == selected))
            .unwrap_or(0);

        let next_position = if delta.is_negative() {
            current_position.saturating_sub(delta.unsigned_abs())
        } else {
            (current_position + delta as usize).min(visible.len().saturating_sub(1))
        };

        self.selected_row = Some(visible[next_position]);
    }
}

fn compare_entries(left: &EntrySummary, right: &EntrySummary, column: SortColumn) -> Ordering {
    match column {
        SortColumn::StartedAt => left.started_at.cmp(&right.started_at),
        SortColumn::Method => left.method.cmp(&right.method),
        SortColumn::Host => left.host.cmp(&right.host),
        SortColumn::Path => left.path.cmp(&right.path),
        SortColumn::Status => left.status.cmp(&right.status),
        SortColumn::Mime => left.mime.cmp(&right.mime),
        SortColumn::ReqBytes => left.req_bytes.cmp(&right.req_bytes),
        SortColumn::ResBytes => left.res_bytes.cmp(&right.res_bytes),
        SortColumn::Duration => left
            .duration_ms
            .partial_cmp(&right.duration_ms)
            .unwrap_or(Ordering::Equal),
    }
}
