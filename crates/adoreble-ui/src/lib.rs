//! Adoreble Wi-Fi — WASM/thin UI glue (ADO-0 placeholder exports).

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn adoreble_ui_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[wasm_bindgen]
pub fn adoreble_ui_http_base() -> String {
    "http://127.0.0.1:9770".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_set() {
        assert!(!adoreble_ui_version().is_empty());
        assert!(adoreble_ui_http_base().contains("9770"));
    }
}
