mod app;
mod components;
mod format;
mod pages;
mod platform;
mod repo;

fn main() {
    console_error_panic_hook::set_once();
    platform::init();
    leptos::mount::mount_to_body(app::App);
}
