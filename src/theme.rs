use eframe::egui;

// --- Premium Dark Slate Palette Constants (eDirStat Identity) ---
pub const BG_PANEL_SLATE: egui::Color32 = egui::Color32::from_rgb(18, 20, 28);
pub const BG_WINDOW_SLATE: egui::Color32 = egui::Color32::from_rgb(26, 29, 38);
pub const STROKE_BORDER_SLATE: egui::Color32 = egui::Color32::from_rgb(38, 43, 56);

// Functional Status Indicators (Used sparingly, matching eDirStat)
pub const COLOR_SEARCHING: egui::Color32 = egui::Color32::from_rgb(139, 92, 246); // Purple: Active background thread processing only
pub const COLOR_STATUS_SUCCESS: egui::Color32 = egui::Color32::from_rgb(34, 197, 94); // Green: Completed actions / successful state
pub const COLOR_MATCH_HIGHLIGHT: egui::Color32 = egui::Color32::from_rgb(245, 158, 11); // Amber: Selected target focus line

pub fn apply_theme(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();

    // Background Surface Fills
    visuals.panel_fill = BG_PANEL_SLATE;
    visuals.window_fill = BG_WINDOW_SLATE;

    // Non-interactive structural rules
    visuals.widgets.noninteractive.bg_fill = BG_WINDOW_SLATE;
    visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, STROKE_BORDER_SLATE);

    // Subtle crisp border curves for interaction elements
    visuals.widgets.inactive.corner_radius = egui::CornerRadius::same(4);
    visuals.widgets.hovered.corner_radius = egui::CornerRadius::same(4);
    visuals.widgets.active.corner_radius = egui::CornerRadius::same(4);

    ctx.set_visuals(visuals);
}
