mod perf;
mod api;
mod app;
mod components;
mod ctx;
mod heads;
mod monitor_bar;
mod palette;
mod prefs;
mod shortcuts;
mod tile_menu;
mod views;

fn main() {
    perf::mark("main");
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(app::App);
    perf::mark("mounted");
}
