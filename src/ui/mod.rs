use std::sync::Arc;

use gloo_file::futures::read_as_bytes;
use leptos::ev::{DragEvent, Event, KeyboardEvent, MouseEvent};
use leptos::html;
use leptos::prelude::*;
use wasm_bindgen_futures::spawn_local;
use web_sys::{File, HtmlElement, HtmlInputElement};

use crate::filter::{FilterQuery, StatusGroup};
use crate::har::HarIndexer;
use crate::state::{HarStore, InspectorTab, SortColumn, SortDirection};

const ROW_HEIGHT: f64 = 30.0;
const ROW_OVERSCAN: usize = 10;

#[component]
pub fn App() -> impl IntoView {
    let store = RwSignal::new(HarStore::default());
    let top_ratio = RwSignal::new(0.55_f64);
    let dragging_main_split = RwSignal::new(false);
    let workspace_ref = NodeRef::<html::Div>::new();
    let file_input_ref = NodeRef::<html::Input>::new();

    let on_main_mousemove = move |ev: MouseEvent| {
        if !dragging_main_split.get() {
            return;
        }

        let Some(container) = workspace_ref.get() else {
            return;
        };

        let rect = container.get_bounding_client_rect();
        let y = f64::from(ev.client_y()) - rect.top();
        let ratio = (y / rect.height()).clamp(0.2, 0.8);
        top_ratio.set(ratio);
    };

    let on_main_mouseup = move |_ev: MouseEvent| {
        dragging_main_split.set(false);
    };

    let on_file_change = {
        let store = store;
        move |ev: Event| {
            let input = event_target::<HtmlInputElement>(&ev);
            if let Some(files) = input.files() {
                if let Some(file) = files.get(0) {
                    load_har_file(file, store);
                }
            }
        }
    };

    let on_drop = {
        let store = store;
        move |ev: DragEvent| {
            ev.prevent_default();
            if let Some(data_transfer) = ev.data_transfer() {
                if let Some(files) = data_transfer.files() {
                    if let Some(file) = files.get(0) {
                        load_har_file(file, store);
                    }
                }
            }
        }
    };

    view! {
        <div class="app-shell" on:dragover=move |ev: DragEvent| ev.prevent_default() on:drop=on_drop>
            <Toolbar store=store file_input_ref=file_input_ref on_file_change=on_file_change />
            <div
                class="workspace"
                node_ref=workspace_ref
                on:mousemove=on_main_mousemove
                on:mouseup=on_main_mouseup
                on:mouseleave=on_main_mouseup
            >
                <div class="history-pane" style=move || format!("height: {}%;", top_ratio.get() * 100.0)>
                    <HistoryPane store=store />
                </div>
                <div
                    class="splitter horizontal"
                    on:mousedown=move |_ev: MouseEvent| dragging_main_split.set(true)
                ></div>
                <div class="inspector-pane">
                    <InspectorPane store=store />
                </div>
            </div>
            <StatusBar store=store />
        </div>
    }
}

#[component]
fn Toolbar(
    store: RwSignal<HarStore>,
    file_input_ref: NodeRef<html::Input>,
    on_file_change: impl Fn(Event) + 'static + Copy,
) -> impl IntoView {
    let open_picker = move |_ev: MouseEvent| {
        if let Some(input) = file_input_ref.get() {
            input.click();
        }
    };

    let on_query_change = {
        let store = store;
        move |ev: Event| {
            let value = event_target_value(&ev);
            store.update(|s| s.filter.text = value);
        }
    };

    let on_method_change = {
        let store = store;
        move |ev: Event| {
            let method = event_target_value(&ev);
            store.update(|s| {
                s.filter.method = (!method.is_empty()).then_some(method);
            });
        }
    };

    let on_status_change = {
        let store = store;
        move |ev: Event| {
            let value = event_target_value(&ev);
            store.update(|s| {
                s.filter.status_group = match value.as_str() {
                    "1xx" => Some(StatusGroup::Informational),
                    "2xx" => Some(StatusGroup::Success),
                    "3xx" => Some(StatusGroup::Redirect),
                    "4xx" => Some(StatusGroup::ClientError),
                    "5xx" => Some(StatusGroup::ServerError),
                    _ => None,
                };
            });
        }
    };

    let on_mime_change = {
        let store = store;
        move |ev: Event| {
            let value = event_target_value(&ev);
            store.update(|s| {
                s.filter.mime_category = (!value.is_empty()).then_some(value);
            });
        }
    };

    let on_min_duration = {
        let store = store;
        move |ev: Event| {
            let value = event_target_value(&ev);
            store.update(|s| {
                s.filter.duration_min = value.parse::<f64>().ok();
            });
        }
    };

    let on_max_duration = {
        let store = store;
        move |ev: Event| {
            let value = event_target_value(&ev);
            store.update(|s| {
                s.filter.duration_max = value.parse::<f64>().ok();
            });
        }
    };

    let clear_filters = {
        let store = store;
        move |_ev: MouseEvent| {
            store.update(|s| s.filter = FilterQuery::default());
        }
    };

    view! {
        <header class="toolbar">
            <div class="toolbar-group">
                <button class="btn" on:click=open_picker>
                    "Import HAR"
                </button>
                <input
                    type="file"
                    accept=".har,.json,application/json"
                    class="hidden-file"
                    node_ref=file_input_ref
                    on:change=on_file_change
                />
                <input
                    type="search"
                    class="search-input"
                    placeholder="Search host/path/method/status"
                    on:input=on_query_change
                />
            </div>

            <div class="toolbar-group compact">
                <select class="field" on:change=on_method_change>
                    <option value="">"All Methods"</option>
                    <option value="GET">"GET"</option>
                    <option value="POST">"POST"</option>
                    <option value="PUT">"PUT"</option>
                    <option value="PATCH">"PATCH"</option>
                    <option value="DELETE">"DELETE"</option>
                </select>

                <select class="field" on:change=on_status_change>
                    <option value="">"All Status"</option>
                    <option value="1xx">"1xx"</option>
                    <option value="2xx">"2xx"</option>
                    <option value="3xx">"3xx"</option>
                    <option value="4xx">"4xx"</option>
                    <option value="5xx">"5xx"</option>
                </select>

                <select class="field" on:change=on_mime_change>
                    <option value="">"All MIME"</option>
                    <option value="application">"application/*"</option>
                    <option value="text">"text/*"</option>
                    <option value="image">"image/*"</option>
                    <option value="font">"font/*"</option>
                </select>

                <input class="field short" type="number" min="0" placeholder="min ms" on:input=on_min_duration />
                <input class="field short" type="number" min="0" placeholder="max ms" on:input=on_max_duration />
                <button class="btn ghost" on:click=clear_filters>
                    "Reset"
                </button>
            </div>
        </header>
    }
}

#[component]
fn HistoryPane(store: RwSignal<HarStore>) -> impl IntoView {
    let scroll_top = RwSignal::new(0.0_f64);
    let viewport_height = RwSignal::new(320.0_f64);
    let visible_indices = Memo::new(move |_| store.with(|s| s.visible_indices()));

    let on_scroll = move |ev: Event| {
        let target = event_target::<HtmlElement>(&ev);
        scroll_top.set(f64::from(target.scroll_top()));
        viewport_height.set(f64::from(target.client_height()));
    };

    let on_keydown = {
        let store = store;
        move |ev: KeyboardEvent| match ev.key().as_str() {
            "ArrowUp" => {
                ev.prevent_default();
                let visible = visible_indices.get();
                store.update(|s| s.move_selection(-1, &visible));
                load_selected_detail(store);
            }
            "ArrowDown" => {
                ev.prevent_default();
                let visible = visible_indices.get();
                store.update(|s| s.move_selection(1, &visible));
                load_selected_detail(store);
            }
            _ => {}
        }
    };

    let rows = move || {
        let visible = visible_indices.get();
        let total = visible.len();
        let total_height = ROW_HEIGHT * total as f64;
        let start = (scroll_top.get() / ROW_HEIGHT).floor().max(0.0) as usize;
        let start = start.saturating_sub(ROW_OVERSCAN);
        let visible_count =
            (viewport_height.get() / ROW_HEIGHT).ceil().max(1.0) as usize + (ROW_OVERSCAN * 2);
        let end = (start + visible_count).min(total);
        let y_offset = start as f64 * ROW_HEIGHT;

        let row_views = visible[start..end]
            .iter()
            .map(|entry_idx| {
                let idx = *entry_idx;
                let row = store.with(|s| s.entries[idx].clone());
                let is_selected = move || store.with(|s| s.selected_row == Some(idx));
                let store_for_click = store;

                view! {
                    <button
                        class="history-row"
                        class:selected=is_selected
                        on:click=move |_ev: MouseEvent| {
                            store_for_click.update(|s| s.selected_row = Some(idx));
                            load_selected_detail(store_for_click);
                        }
                    >
                        <span class="col method">{row.method}</span>
                        <span class="col host">{row.host}</span>
                        <span class="col path">{row.path}</span>
                        <span class="col status">{row.status}</span>
                        <span class="col mime">{row.mime}</span>
                        <span class="col bytes">{format_bytes(row.res_bytes)}</span>
                        <span class="col time">{row.started_at}</span>
                        <span class="col duration">{format!("{:.1} ms", row.duration_ms)}</span>
                    </button>
                }
            })
            .collect_view();

        view! {
            <div class="history-inner" style=move || format!("height: {total_height}px;")>
                <div class="history-virtual" style=move || format!("transform: translateY({y_offset}px);")>
                    {row_views}
                </div>
            </div>
        }
    };

    view! {
        <div class="history-root">
            <div class="history-header">
                <SortButton label="Method" column=SortColumn::Method store=store />
                <SortButton label="Host" column=SortColumn::Host store=store />
                <SortButton label="Path" column=SortColumn::Path store=store />
                <SortButton label="Status" column=SortColumn::Status store=store />
                <SortButton label="MIME" column=SortColumn::Mime store=store />
                <SortButton label="Resp Size" column=SortColumn::ResBytes store=store />
                <SortButton label="Started" column=SortColumn::StartedAt store=store />
                <SortButton label="Duration" column=SortColumn::Duration store=store />
            </div>
            <div class="history-scroll" tabindex="0" on:scroll=on_scroll on:keydown=on_keydown>
                {rows}
            </div>
        </div>
    }
}

#[component]
fn SortButton(label: &'static str, column: SortColumn, store: RwSignal<HarStore>) -> impl IntoView {
    let indicator = move || {
        store.with(|s| {
            if s.sort.column != column {
                String::new()
            } else {
                match s.sort.direction {
                    SortDirection::Asc => " \u{25b2}".to_string(),
                    SortDirection::Desc => " \u{25bc}".to_string(),
                }
            }
        })
    };

    view! {
        <button class="sort-btn" on:click=move |_ev: MouseEvent| store.update(|s| s.toggle_sort(column))>
            {move || format!("{}{}", label, indicator())}
        </button>
    }
}

#[component]
fn InspectorPane(store: RwSignal<HarStore>) -> impl IntoView {
    let side_ratio = RwSignal::new(0.32_f64);
    let dragging_side_split = RwSignal::new(false);
    let inspector_ref = NodeRef::<html::Div>::new();

    let on_mousemove = move |ev: MouseEvent| {
        if !dragging_side_split.get() {
            return;
        }
        let Some(panel) = inspector_ref.get() else {
            return;
        };
        let rect = panel.get_bounding_client_rect();
        let x = f64::from(ev.client_x()) - rect.left();
        let ratio = (x / rect.width()).clamp(0.2, 0.5);
        side_ratio.set(ratio);
    };

    let on_mouseup = move |_ev: MouseEvent| {
        dragging_side_split.set(false);
    };

    let set_tab = {
        let store = store;
        move |tab: InspectorTab| {
            store.update(|s| s.active_tab = tab);
            load_selected_detail(store);
        }
    };

    let summary_panel = move || {
        let summary = store.with(|s| s.selected_summary().cloned());
        match summary {
            Some(row) => view! {
                <div class="summary-card">
                    <h3>{format!("{} {}", row.method, row.path)}</h3>
                    <dl>
                        <dt>"Host"</dt><dd>{row.host}</dd>
                        <dt>"Status"</dt><dd>{row.status}</dd>
                        <dt>"MIME"</dt><dd>{row.mime}</dd>
                        <dt>"Request"</dt><dd>{format_bytes(row.req_bytes)}</dd>
                        <dt>"Response"</dt><dd>{format_bytes(row.res_bytes)}</dd>
                        <dt>"Duration"</dt><dd>{format!("{:.1} ms", row.duration_ms)}</dd>
                    </dl>
                </div>
            }
            .into_any(),
            None => view! { <div class="summary-empty">"No entry selected"</div> }.into_any(),
        }
    };

    let detail_content = move || {
        let active_tab = store.with(|s| s.active_tab);
        let detail = store.with(|s| s.selected_detail().cloned());

        match (active_tab, detail) {
            (_, None) => view! { <div class="empty-detail">"Select an entry to inspect request/response data."</div> }.into_any(),
            (InspectorTab::Request, Some(detail)) => view! {
                <div class="detail-grid">
                    <h4>{detail.request_line}</h4>
                    <pre class="code-view">{format_headers(&detail.request_headers)}</pre>
                    <pre class="code-view body">{detail.request_body}</pre>
                </div>
            }
            .into_any(),
            (InspectorTab::Response, Some(detail)) => view! {
                <div class="detail-grid">
                    <h4>{format!("{} {}", detail.response_status, detail.response_reason)}</h4>
                    <pre class="code-view">{format_headers(&detail.response_headers)}</pre>
                    <pre class="code-view body">{detail.response_body}</pre>
                </div>
            }
            .into_any(),
            (InspectorTab::Headers, Some(detail)) => view! {
                <div class="headers-pair">
                    <div>
                        <h4>"Request Headers"</h4>
                        <pre class="code-view">{format_headers(&detail.request_headers)}</pre>
                    </div>
                    <div>
                        <h4>"Response Headers"</h4>
                        <pre class="code-view">{format_headers(&detail.response_headers)}</pre>
                    </div>
                </div>
            }
            .into_any(),
            (InspectorTab::Timings, Some(detail)) => view! {
                <table class="timing-table">
                    <tbody>
                        <tr><th>"Blocked"</th><td>{format!("{:.2}", detail.timings.blocked)}</td></tr>
                        <tr><th>"DNS"</th><td>{format!("{:.2}", detail.timings.dns)}</td></tr>
                        <tr><th>"Connect"</th><td>{format!("{:.2}", detail.timings.connect)}</td></tr>
                        <tr><th>"SSL"</th><td>{format!("{:.2}", detail.timings.ssl)}</td></tr>
                        <tr><th>"Send"</th><td>{format!("{:.2}", detail.timings.send)}</td></tr>
                        <tr><th>"Wait"</th><td>{format!("{:.2}", detail.timings.wait)}</td></tr>
                        <tr><th>"Receive"</th><td>{format!("{:.2}", detail.timings.receive)}</td></tr>
                    </tbody>
                </table>
            }
            .into_any(),
        }
    };

    view! {
        <div
            class="inspector-layout"
            node_ref=inspector_ref
            on:mousemove=on_mousemove
            on:mouseup=on_mouseup
            on:mouseleave=on_mouseup
        >
            <div class="inspector-sidebar" style=move || format!("width: {}%;", side_ratio.get() * 100.0)>
                {summary_panel}
            </div>
            <div class="splitter vertical" on:mousedown=move |_ev: MouseEvent| dragging_side_split.set(true)></div>
            <div class="inspector-main">
                <div class="tab-row">
                    <button class="tab" class:active=move || store.with(|s| s.active_tab == InspectorTab::Request) on:click=move |_ev: MouseEvent| set_tab(InspectorTab::Request)>"Request"</button>
                    <button class="tab" class:active=move || store.with(|s| s.active_tab == InspectorTab::Response) on:click=move |_ev: MouseEvent| set_tab(InspectorTab::Response)>"Response"</button>
                    <button class="tab" class:active=move || store.with(|s| s.active_tab == InspectorTab::Headers) on:click=move |_ev: MouseEvent| set_tab(InspectorTab::Headers)>"Headers"</button>
                    <button class="tab" class:active=move || store.with(|s| s.active_tab == InspectorTab::Timings) on:click=move |_ev: MouseEvent| set_tab(InspectorTab::Timings)>"Timings"</button>
                </div>
                <div class="tab-content">{detail_content}</div>
            </div>
        </div>
    }
}

#[component]
fn StatusBar(store: RwSignal<HarStore>) -> impl IntoView {
    let text = move || {
        store.with(|s| {
            if let Some(error) = &s.error {
                return format!("Error: {error}");
            }
            if s.indexing {
                return format!("Indexing HAR... {:.0}%", s.indexing_progress * 100.0);
            }

            match s.stats {
                Some(stats) => format!(
                    "Entries: {} | Indexed bytes: {} | Visible: {}",
                    stats.entry_count,
                    format_bytes(stats.indexed_bytes as u64),
                    s.visible_indices().len()
                ),
                None => "Drop a HAR file to begin.".to_string(),
            }
        })
    };

    view! { <footer class="status-bar">{text}</footer> }
}

fn load_har_file(file: File, store: RwSignal<HarStore>) {
    spawn_local(async move {
        store.update(|s| s.begin_indexing());

        let bytes = match read_as_bytes(&gloo_file::File::from(file)).await {
            Ok(bytes) => bytes,
            Err(error) => {
                store.update(|s| s.set_error(format!("Failed reading file: {error}")));
                return;
            }
        };

        let bytes_arc: Arc<[u8]> = bytes.into();

        let index_result = HarIndexer::index_cooperative(bytes_arc.as_ref(), |done, total| {
            let progress = if total == 0 {
                1.0
            } else {
                done as f32 / total as f32
            };
            store.update(|s| s.indexing_progress = progress);
        })
        .await;

        match index_result {
            Ok(result) => {
                store.update(|s| s.set_index_result(bytes_arc, result));
                load_selected_detail(store);
            }
            Err(error) => {
                store.update(|s| s.set_error(error.to_string()));
            }
        }
    });
}

fn load_selected_detail(store: RwSignal<HarStore>) {
    let maybe_load = store.with(|s| {
        let selected = s.selected_row?;
        if s.details.contains_key(&selected) {
            return None;
        }
        let bytes = s.file_bytes.clone()?;
        let range = *s.entry_ranges.get(selected)?;
        Some((selected, bytes, range))
    });

    let Some((selected, bytes, range)) = maybe_load else {
        return;
    };

    match HarIndexer::load_detail(bytes.as_ref(), range) {
        Ok(detail) => {
            store.update(|s| {
                s.details.insert(selected, detail);
            });
        }
        Err(error) => {
            store.update(|s| s.set_error(error.to_string()));
        }
    }
}

fn format_headers(headers: &[(String, String)]) -> String {
    headers
        .iter()
        .map(|(name, value)| format!("{name}: {value}"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn format_bytes(value: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;

    let bytes = value as f64;
    if bytes >= GB {
        format!("{:.2} GB", bytes / GB)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes / MB)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes / KB)
    } else {
        format!("{} B", value)
    }
}
