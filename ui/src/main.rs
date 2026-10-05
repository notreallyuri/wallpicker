mod api;
mod app;
mod components;
mod ctx;
mod monitor_bar;
mod prefs;
mod shortcuts;
mod tile_menu;
mod views;

fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(app::App);
}
