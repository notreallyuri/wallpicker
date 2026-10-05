//! Small view preferences kept in the webview's localStorage (sort, layout).
//! Conveniences only: when storage isn't available, the defaults are used.

fn storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok()?
}

pub(crate) fn get(key: &str) -> Option<String> {
    storage()?.get_item(&format!("wallpicker.{key}")).ok()?
}

pub(crate) fn set(key: &str, value: &str) {
    if let Some(s) = storage() {
        let _ = s.set_item(&format!("wallpicker.{key}"), value);
    }
}
