mod core_client;
pub mod commands;
pub mod design;
mod document;
pub mod empty_state;
mod i18n;
pub mod icons;
pub mod languages;
pub mod workspace_config;

use gpui::{AbsoluteLength, Pixels, rems};

rust_i18n::i18n!("locales", fallback = "en");

pub use core_client::{
    CoreClient, CoreError, CoreRequest, DEFAULT_TIMEOUT_MILLIS, cancel_operation, core_json,
};
pub use document::{
    DOCUMENT_VERSION_KEY, Parsed, declared_version, known_keys, merge_document, parse,
    preserve_unknown, previous_object, read_document_text, save_document, save_json, tmp_path,
};
pub use i18n::{tr, tr_args};

pub const HOST_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn rem_px(rem: Pixels, spec_px: f32) -> Pixels {
    AbsoluteLength::from(rems(spec_px / 16.)).to_pixels(rem)
}

#[cfg(test)]
mod tests {
    use super::rem_px;
    use gpui::px;

    #[test]
    fn rem_px_scales_with_the_runtime_base() {
        assert_eq!(rem_px(px(16.), 26.), px(26.));
        assert_eq!(rem_px(px(32.), 26.), px(52.));
        assert_eq!(rem_px(px(20.), 32.), px(40.));
        assert_eq!(rem_px(px(16.), 1.6), px(1.6));
        assert_eq!(rem_px(px(32.), 200.), px(400.));
    }
}
