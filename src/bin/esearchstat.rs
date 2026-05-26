// -- Clippy Denies --
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// --- Clippy Lint Groups & Specific Warnings ---
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![warn(clippy::cargo)]
#![warn(clippy::needless_return)]
// --- Allowed Lints (Overrides) ---
#![allow(clippy::mod_module_files)]
#![allow(clippy::unseparated_literal_suffix)]
#![allow(clippy::missing_inline_in_public_items)]
#![allow(clippy::panic)]
#![allow(clippy::multiple_crate_versions)]
#![allow(clippy::blanket_clippy_restriction_lints)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::cast_lossless)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cognitive_complexity)]
#![allow(clippy::cargo_common_metadata)]
#![allow(clippy::future_not_send)]
#![allow(clippy::used_underscore_binding)]
#![allow(clippy::crate_in_macro_def)]
#![allow(clippy::too_many_lines)]

fn main() -> eframe::Result {
    let native_options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("eSearchStat")
            .with_inner_size([1100.0, 750.0])
            .with_min_inner_size([800.0, 550.0]),
        ..Default::default()
    };
    eframe::run_native(
        "esearchstat",
        native_options,
        Box::new(|cc| {
            esearchstat::theme::apply_theme(&cc.egui_ctx);
            Ok(Box::new(esearchstat::gui::SearchApp::default()))
        }),
    )
}
