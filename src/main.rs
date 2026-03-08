#[cfg(target_arch = "wasm32")]
fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(har_viewer::ui::App);
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    println!("This app is intended to run in the browser via `trunk serve`.");
}
