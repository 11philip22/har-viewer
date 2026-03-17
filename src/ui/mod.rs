use std::sync::Arc;

use gloo_file::futures::read_as_bytes;
use leptos::ev::{DragEvent, Event, KeyboardEvent, MouseEvent};
use leptos::html;
use leptos::prelude::*;
use wasm_bindgen_futures::spawn_local;
use web_sys::{File, HtmlInputElement};

use crate::filter::{FilterQuery, StatusGroup};
use crate::har::{HarIndexer, build_request_message, build_response_message};
use crate::state::{HarStore, SortColumn, SortDirection};

#[component]
pub fn App() -> impl IntoView {
    let store = RwSignal::new(HarStore::default());
    let theme = RwSignal::new(load_theme());
    let top_ratio = RwSignal::new(0.55_f64);
    let dragging_main_split = RwSignal::new(false);
    let workspace_ref = NodeRef::<html::Div>::new();
    let file_input_ref = NodeRef::<html::Input>::new();

    create_effect(move |_| {
        let value = theme.get();
        apply_theme(&value);
    });

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
            <Toolbar store=store file_input_ref=file_input_ref on_file_change=on_file_change theme=theme />
            <div
                class="workspace"
                node_ref=workspace_ref
                style=move || format!("grid-template-rows: {}% 8px minmax(180px, 1fr);", top_ratio.get() * 100.0)
                on:mousemove=on_main_mousemove
                on:mouseup=on_main_mouseup
                on:mouseleave=on_main_mouseup
            >
                <div class="history-pane">
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
    theme: RwSignal<String>,
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
    let clear_filters = {
        let store = store;
        move |_ev: MouseEvent| {
            store.update(|s| s.filter = FilterQuery::default());
        }
    };

    let toggle_theme = {
        let theme = theme;
        move |_ev: MouseEvent| {
            theme.update(|value| {
                if value.as_str() == "light" {
                    *value = "dark".to_string();
                } else {
                    *value = "light".to_string();
                }
            });
        }
    };

    view! {
        <header class="toolbar texture-void">
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
                <button class="btn ghost" on:click=clear_filters>
                    "Reset"
                </button>
                <button class="theme-toggle" on:click=toggle_theme>
                    {move || {
                        if theme.get() == "light" {
                            "Theme: Light"
                        } else {
                            "Theme: Dark"
                        }
                    }}
                </button>
            </div>
        </header>
    }
}

#[component]
fn HistoryPane(store: RwSignal<HarStore>) -> impl IntoView {
    let visible_indices = Memo::new(move |_| store.with(|s| s.visible_indices()));

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

    let body_rows = move || {
        let visible = visible_indices.get();
        visible
            .iter()
            .map(|entry_idx| {
                let idx = *entry_idx;
                let row = store.with(|s| s.entries[idx].clone());
                let method = row.method;
                let method_title = method.clone();
                let host = row.host;
                let host_title = host.clone();
                let path = row.path;
                let path_title = path.clone();
                let status = row.status;
                let status_title = status.to_string();
                let mime = row.mime;
                let mime_title = mime.clone();
                let started_at = row.started_at;
                let started_at_title = started_at.clone();
                let bytes_text = format_bytes(row.res_bytes);
                let duration_text = format!("{:.1} ms", row.duration_ms);

                let is_selected = move || store.with(|s| s.selected_row == Some(idx));
                let store_for_click = store;

                view! {
                    <tr
                        class="history-row"
                        class:selected=is_selected
                        on:click=move |_ev: MouseEvent| {
                            store_for_click.update(|s| s.selected_row = Some(idx));
                            load_selected_detail(store_for_click);
                        }
                    >
                        <td class="col method"><span class="cell-truncate" title=method_title>{method}</span></td>
                        <td class="col host"><span class="cell-truncate" title=host_title>{host}</span></td>
                        <td class="col path"><span class="cell-truncate" title=path_title>{path}</span></td>
                        <td class="col status"><span class="cell-truncate" title=status_title>{status}</span></td>
                        <td class="col mime"><span class="cell-truncate" title=mime_title>{mime}</span></td>
                        <td class="col bytes"><span class="cell-truncate">{bytes_text}</span></td>
                        <td class="col time"><span class="cell-truncate" title=started_at_title>{started_at}</span></td>
                        <td class="col duration"><span class="cell-truncate">{duration_text}</span></td>
                    </tr>
                }
            })
            .collect_view()
    };

    view! {
        <div class="history-root texture-scan">
            <div class="history-scroll" tabindex="0" on:keydown=on_keydown>
                <table class="history-table">
                    <colgroup>
                        <col class="col-method" />
                        <col class="col-host" />
                        <col class="col-path" />
                        <col class="col-status" />
                        <col class="col-mime" />
                        <col class="col-bytes" />
                        <col class="col-time" />
                        <col class="col-duration" />
                    </colgroup>
                    <thead>
                        <tr>
                            <th><SortButton label="Method" column=SortColumn::Method store=store /></th>
                            <th><SortButton label="Host" column=SortColumn::Host store=store /></th>
                            <th><SortButton label="Path" column=SortColumn::Path store=store /></th>
                            <th><SortButton label="Status" column=SortColumn::Status store=store /></th>
                            <th><SortButton label="MIME" column=SortColumn::Mime store=store /></th>
                            <th><SortButton label="Resp Size" column=SortColumn::ResBytes store=store /></th>
                            <th><SortButton label="Started" column=SortColumn::StartedAt store=store /></th>
                            <th><SortButton label="Duration" column=SortColumn::Duration store=store /></th>
                        </tr>
                    </thead>
                    <tbody>{body_rows}</tbody>
                </table>
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
    let request_message = move || {
        store.with(|s| {
            s.selected_detail()
                .map(build_request_message)
                .unwrap_or_else(|| "Select an entry to inspect request data.".to_string())
        })
    };

    let response_message = move || {
        store.with(|s| {
            s.selected_detail()
                .map(build_response_message)
                .unwrap_or_else(|| "Select an entry to inspect response data.".to_string())
        })
    };

    view! {
        <div class="message-split">
            <section class="message-panel texture-scan">
                <h4 class="message-title">"request:"</h4>
                <pre class="message-view">{request_message}</pre>
            </section>
            <section class="message-panel texture-scan">
                <h4 class="message-title">"response:"</h4>
                <pre class="message-view">{response_message}</pre>
            </section>
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

    view! { <footer class="status-bar texture-void">{text}</footer> }
}

fn load_theme() -> String {
    let Some(window) = web_sys::window() else {
        return "dark".to_string();
    };

    if let Ok(Some(storage)) = window.local_storage() {
        if let Ok(Some(value)) = storage.get_item("har-viewer-theme") {
            if value == "light" {
                return value;
            }
        }
    }

    "dark".to_string()
}

fn apply_theme(theme: &str) {
    if let Some(window) = web_sys::window() {
        if let Some(document) = window.document() {
            if let Some(root) = document.document_element() {
                let _ = root.set_attribute("data-theme", theme);
            }
        }
        if let Ok(Some(storage)) = window.local_storage() {
            let _ = storage.set_item("har-viewer-theme", theme);
        }
    }
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






