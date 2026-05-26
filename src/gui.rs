use std::path::{Path, PathBuf};

use crossbeam_channel::Receiver;
use eframe::egui;

use super::{
    core::{FilePreviewer, SearchMatch, start_search},
    storage::{load_results, save_results},
    theme::{COLOR_MATCH_HIGHLIGHT, COLOR_SEARCHING, COLOR_STATUS_SUCCESS, STROKE_BORDER_SLATE},
    tree::{FlatRow, SearchTree},
};

pub struct SearchApp {
    pub query: String,
    pub directory: String,
    pub matches: Vec<SearchMatch>,
    pub rx: Option<Receiver<SearchMatch>>,

    // Checkboxes & limits
    pub ignore_case: bool,
    pub read_hidden: bool,
    pub show_tree: bool,
    pub monospaced: bool,
    pub show_about: bool,
    pub limit: usize,

    // Tree structure
    pub tree: SearchTree,

    // Streaming previewer modal
    pub previewer: Option<FilePreviewer>,

    // Save and load results fields
    pub error_message: Option<String>,
    pub is_searching: bool,
}

impl Default for SearchApp {
    fn default() -> Self {
        Self {
            query: String::new(),
            directory: String::new(),
            matches: Vec::new(),
            rx: None,
            ignore_case: false,
            read_hidden: false,
            show_tree: false,
            monospaced: false,
            show_about: false,
            limit: 50_000,
            tree: SearchTree::default(),
            previewer: None,
            error_message: None,
            is_searching: false,
        }
    }
}

impl SearchApp {
    pub fn open_preview(&mut self, path: PathBuf, line: usize) {
        self.previewer = Some(FilePreviewer::new(path, line));
    }

    pub fn open_parent_in_explorer(&self, path: &Path) {
        if let Some(parent) = path.parent() {
            let _ = open::that(parent);
        }
    }

    pub fn start_search_action(&mut self, ctx: &egui::Context) {
        self.matches.clear();
        self.is_searching = true;
        self.tree = SearchTree::default();
        self.tree.path = PathBuf::from(&self.directory);

        let (tx, rx) = crossbeam_channel::unbounded();
        self.rx = Some(rx);

        let path = PathBuf::from(&self.directory);
        start_search(
            self.query.clone(),
            path,
            self.ignore_case,
            self.read_hidden,
            self.limit,
            tx,
            ctx.clone(),
        );
    }

    pub fn save_results_action(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("eSearchStat Search Results", &["ess"])
            .set_file_name("search_results.ess")
            .save_file()
        {
            if let Err(e) = save_results(&self.matches, &path) {
                self.error_message = Some(format!("Failed to save results: {e}"));
            } else {
                let path_display = path.display();
                self.error_message = Some(format!("Successfully saved to {path_display}"));
            }
        }
    }

    pub fn load_results_action(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("eSearchStat Search Results", &["ess"])
            .pick_file()
        {
            match load_results(&path) {
                Ok(matches) => {
                    self.matches = matches;
                    // Rebuild tree
                    self.tree = SearchTree::default();
                    self.tree.path = PathBuf::from(&self.directory);
                    for m in &self.matches {
                        let rel_path = match m.path.strip_prefix(&self.directory) {
                            Ok(p) => p,
                            Err(_) => &m.path,
                        };
                        let comps: Vec<String> = rel_path
                            .components()
                            .map(|c| c.as_os_str().to_string_lossy().into_owned())
                            .collect();
                        self.tree.insert(m.clone(), &comps);
                    }
                    self.error_message = Some(format!(
                        "Successfully loaded {} matches",
                        self.matches.len()
                    ));
                }
                Err(e) => {
                    self.error_message = Some(format!("Failed to load results: {e}"));
                }
            }
        }
    }

    pub fn export_results_action(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Text Files", &["txt"])
            .set_file_name("search_results.txt")
            .save_file()
        {
            match export_to_text(&self.matches, &path) {
                Ok(()) => {
                    let path_display = path.display();
                    self.error_message = Some(format!("Successfully exported to {path_display}"));
                }
                Err(e) => {
                    self.error_message = Some(format!("Failed to export results: {e}"));
                }
            }
        }
    }

    pub fn copy_command_action(&mut self, ctx: &egui::Context) {
        let mut args = Vec::new();
        if self.ignore_case {
            args.push("-i".to_string());
        }
        if self.read_hidden {
            args.push("--hidden".to_string());
        }
        args.push(format!("-m {}", self.limit));

        let escaped_query = self.query.replace('\'', "'\\''");
        args.push(format!("'{escaped_query}'"));

        let dir = if self.directory.is_empty() {
            ".".to_string()
        } else {
            self.directory.clone()
        };
        let escaped_dir = dir.replace('\'', "'\\''");
        args.push(format!("'{escaped_dir}'"));

        let rg_cmd = format!("rg {}", args.join(" "));
        ctx.copy_text(rg_cmd);
        self.error_message = Some("Copied ripgrep command to clipboard!".to_string());
    }

    pub fn chain_search_action(&mut self) {
        use regex::RegexBuilder;

        let re = match RegexBuilder::new(&self.query)
            .case_insensitive(self.ignore_case)
            .build()
        {
            Ok(re) => re,
            Err(e) => {
                self.error_message = Some(format!("Invalid regular expression: {e}"));
                return;
            }
        };

        let old_len = self.matches.len();
        self.matches.retain(|m| re.is_match(&m.line_content));

        // Rebuild tree
        self.tree = SearchTree::default();
        self.tree.path = PathBuf::from(&self.directory);
        for m in &self.matches {
            let rel_path = match m.path.strip_prefix(&self.directory) {
                Ok(p) => p,
                Err(_) => &m.path,
            };
            let comps: Vec<String> = rel_path
                .components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect();
            self.tree.insert(m.clone(), &comps);
        }

        self.error_message = Some(format!(
            "Chained search complete: retained {} of {} matches",
            self.matches.len(),
            old_len
        ));
    }
}

impl eframe::App for SearchApp {
    fn ui(&mut self, ui: &mut eframe::egui::Ui, _frame: &mut eframe::Frame) {
        // Apply theme continuously to ensure stable colors
        crate::theme::apply_theme(ui.ctx());

        // 1. Drain any matches that arrived in the channel since the last frame
        let mut new_matches_received = false;
        if let Some(ref rx) = self.rx {
            while let Ok(search_match) = rx.try_recv() {
                new_matches_received = true;
                self.matches.push(search_match.clone());
                let rel_path = match search_match.path.strip_prefix(&self.directory) {
                    Ok(p) => p,
                    Err(_) => &search_match.path,
                };
                let comps: Vec<String> = rel_path
                    .components()
                    .map(|c| c.as_os_str().to_string_lossy().into_owned())
                    .collect();
                self.tree.insert(search_match, &comps);
            }
        }

        // Manage active background scanning indicators natively
        if self.is_searching
            && !new_matches_received
            && self
                .rx
                .as_ref()
                .is_none_or(crossbeam_channel::Receiver::is_empty)
        {
            self.is_searching = false;
        }

        // 2. Top Control Panel with flat menu and sleek horizontal query deck
        egui::Panel::top("top_panel")
            .resizable(false)
            .show_inside(ui, |ui| {
                ui.add_space(2.0); // Perfect top alignment balancing bottom margins
                ui.horizontal(|ui| {
                    ui.heading(
                        egui::RichText::new("eSearchStat 🔍")
                            .strong()
                            .color(ui.visuals().strong_text_color()),
                    );
                    ui.separator();

                    // Temporarily disable button frames to make top-level menus flat & clean
                    let saved_button_frame = ui.visuals().button_frame;
                    ui.style_mut().visuals.button_frame = false;

                    // Top menu buttons (File / View / Help)
                    ui.menu_button("File", |ui| {
                        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);

                        let search_btn_text = if self.is_searching {
                            "⏳ Searching..."
                        } else {
                            "🔍 New Search"
                        };
                        let start_search_resp = ui.add_enabled(
                            !self.is_searching && !self.query.is_empty(),
                            egui::Button::new(search_btn_text),
                        );
                        let start_search_resp = start_search_resp.on_hover_text("Search the filesystem under the specified path using the query pattern");
                        if start_search_resp.clicked() {
                            self.start_search_action(ui.ctx());
                            ui.close_kind(egui::UiKind::Menu);
                        }

                        let chain_enabled = !self.is_searching && !self.matches.is_empty() && !self.query.is_empty();
                        let chain_btn_resp = ui.add_enabled(
                            chain_enabled,
                            egui::Button::new("🔗 Chain Search"),
                        );
                        let chain_btn_resp = chain_btn_resp.on_hover_text("Search within the current search results in-memory, sharing the query but ignoring the path");
                        if chain_btn_resp.clicked() {
                            self.chain_search_action();
                            ui.close_kind(egui::UiKind::Menu);
                        }

                        ui.separator();

                        if ui.button("💾 Save Results...").clicked() {
                            self.save_results_action();
                            ui.close_kind(egui::UiKind::Menu);
                        }

                        if ui.button("📖 Load Results...").clicked() {
                            self.load_results_action();
                            ui.close_kind(egui::UiKind::Menu);
                        }

                        if ui.button("📤 Export Plain Text...").clicked() {
                            self.export_results_action();
                            ui.close_kind(egui::UiKind::Menu);
                        }
                    });

                    ui.menu_button("View", |ui| {
                        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);

                        // Aligned checkbox layout for monospace font
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 4.0;
                            let mut checked = self.monospaced;
                            if ui.checkbox(&mut checked, "").changed() {
                                self.monospaced = checked;
                            }
                            let response = ui
                                .horizontal(|ui| {
                                    ui.label(egui::RichText::new("🅰").size(12.0));
                                    ui.label(
                                        egui::RichText::new("Monospace Font for Matches")
                                            .color(ui.visuals().widgets.inactive.text_color()),
                                    );
                                })
                                .response;

                            let label_click = ui.interact(
                                response.rect,
                                ui.id().with("mono_label"),
                                egui::Sense::click(),
                            );
                            if label_click.clicked() {
                                self.monospaced = !self.monospaced;
                            }
                        });
                    });

                    ui.menu_button("Help", |ui| {
                        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                        if ui.button("ℹ About").clicked() {
                            self.show_about = true;
                            ui.close_kind(egui::UiKind::Menu);
                        }
                    });

                    ui.separator();

                    // Restore default button frames for the primary CTA button
                    ui.style_mut().visuals.button_frame = saved_button_frame;

                    let search_btn_text = if self.is_searching {
                        "⏳ Searching..."
                    } else {
                        "🔍 New Search"
                    };
                    let start_search_resp = ui.add_enabled(
                        !self.is_searching && !self.query.is_empty(),
                        egui::Button::new(search_btn_text),
                    );
                    let start_search_resp = start_search_resp.on_hover_text("Search the filesystem under the specified path using the query pattern");
                    if start_search_resp.clicked() {
                        self.start_search_action(ui.ctx());
                    }

                    let chain_enabled = !self.is_searching && !self.matches.is_empty() && !self.query.is_empty();
                    let chain_btn_resp = ui.add_enabled(
                        chain_enabled,
                        egui::Button::new("🔗 Chain Search"),
                    );
                    let chain_btn_resp = chain_btn_resp.on_hover_text("Search within the current search results in-memory, sharing the query but ignoring the path");
                    if chain_btn_resp.clicked() {
                        self.chain_search_action();
                    }

                    ui.separator();

                    // Live status display
                    if self.is_searching {
                        ui.spinner();
                        ui.colored_label(COLOR_SEARCHING, "Searching Filesystem...");
                        ui.ctx()
                            .request_repaint_after(std::time::Duration::from_millis(50));
                    } else if !self.matches.is_empty() {
                        ui.colored_label(COLOR_STATUS_SUCCESS, "Search Complete");
                    } else {
                        ui.label("Idle");
                    }
                });

                ui.separator();

                // Sleek Search Parameters Control Deck
                egui::Grid::new("search_param_grid")
                    .num_columns(2)
                    .spacing([8.0, 6.0])
                    .show(ui, |ui| {
                        let btn_width = 95.0;

                        ui.label(egui::RichText::new("Query:").strong());
                        ui.horizontal(|ui| {
                            let edit_width = ui.available_width() - btn_width - 8.0;

                            let text_edit = egui::TextEdit::singleline(&mut self.query)
                                .desired_width(edit_width.max(50.0))
                                .hint_text("Enter pattern or regular expression...");
                            let resp = ui.add(text_edit);

                            if resp.lost_focus()
                                && ui.input(|i| i.key_pressed(egui::Key::Enter))
                                && !self.query.is_empty()
                                && !self.is_searching
                            {
                                self.start_search_action(ui.ctx());
                            }

                            let copy_btn =
                                egui::Button::new("📋 Copy").min_size(egui::vec2(btn_width, 0.0));
                            if ui.add(copy_btn).clicked() {
                                self.copy_command_action(ui.ctx());
                            }
                        });
                        ui.end_row();

                        ui.label(egui::RichText::new("Path:").strong());
                        ui.horizontal(|ui| {
                            let edit_width = ui.available_width() - btn_width - 8.0;
                            ui.add(
                                egui::TextEdit::singleline(&mut self.directory)
                                    .desired_width(edit_width.max(50.0))
                                    .hint_text("Directory to search in..."),
                            );
                            let browse_btn =
                                egui::Button::new("📁 Browse").min_size(egui::vec2(btn_width, 0.0));
                            if ui.add(browse_btn).clicked()
                                && let Some(path) = rfd::FileDialog::new().pick_folder()
                            {
                                self.directory = path.to_string_lossy().to_string();
                            }
                        });
                        ui.end_row();
                    });

                ui.add_space(6.0);

                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.ignore_case, "Ignore Case");
                    ui.checkbox(&mut self.read_hidden, "Read Hidden");

                    ui.separator();
                    ui.label("Max Results:");
                    let response = ui.add(
                        egui::DragValue::new(&mut self.limit)
                            .range(100..=1_000_000)
                            .speed(500),
                    );

                    let bytes_per_match = 256;
                    #[allow(clippy::cast_precision_loss)]
                    let est_mb = (self.limit * bytes_per_match) as f64 / (1024.0 * 1024.0);
                    response.on_hover_text(format!(
                        "Est. RAM: {est_mb:.2} MB (average ~{bytes_per_match} bytes/result)"
                    ));
                });

                if let Some(ref msg) = self.error_message {
                    ui.add_space(4.0);
                    let text_color = if msg.contains("Failed") {
                        egui::Color32::from_rgb(239, 68, 68)
                    } else {
                        COLOR_STATUS_SUCCESS
                    };
                    ui.colored_label(text_color, msg);
                }

                ui.add_space(4.0);
            });

        // 3. Bottom Status Bar Panel
        egui::Panel::bottom("bottom_panel").show_inside(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(format!("🔍 Matches found: {}", self.matches.len()));
                if !self.directory.is_empty() {
                    ui.separator();
                    ui.label(format!("📂 Active Search Path: {}", self.directory));
                }

                if self.is_searching {
                    ui.separator();
                    ui.spinner();
                    ui.colored_label(COLOR_SEARCHING, "Scanning files...");
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(format!("Max Limit: {} results", self.limit));
                });
            });
        });

        // 4. Edge-to-Edge Central Panel ensuring scrollbar is on the far right along the window edge
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE) // Zero margins for panel frame
            .show_inside(ui, |ui| {
                // Sleek View Mode Tab Switcher at the top of the Central Panel
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.add_space(8.0);
                    ui.selectable_value(&mut self.show_tree, false, "📝 Flat List View");
                    ui.selectable_value(&mut self.show_tree, true, "🌳 Collapsible Tree View");
                });
                ui.add_space(2.0);
                ui.separator();

                let scroll_area = egui::ScrollArea::vertical().auto_shrink([false, false]); // Stretch to fill all space

                // Deferred actions to prevent mutable borrow conflicts in loops
                let mut open_preview_target = None;
                let mut open_explorer_target = None;

                // Compute the maximum line number digits padding among ALL matches (min 6 digits to prevent streaming search jitter)
                let max_line_number = self
                    .matches
                    .iter()
                    .map(|m| m.line_number)
                    .max()
                    .unwrap_or(0);
                let pad_len = max_line_number.to_string().len().max(6);

                if self.show_tree {
                    let mut flat_rows = Vec::new();
                    self.tree.flatten_impl(0, &mut flat_rows);
                    let total_rows = flat_rows.len();
                    let flat_row_height = ui.spacing().interact_size.y + 4.0;
                    ui.spacing_mut().item_spacing.y = 4.0;

                    let mut toggle_target = None;

                    scroll_area.show_rows(ui, flat_row_height, total_rows, |ui, row_range| {
                        ui.with_layout(egui::Layout::top_down(egui::Align::LEFT), |ui| {
                            for idx in row_range {
                                let row = &flat_rows[idx];
                                match row {
                                    FlatRow::Directory { node, indent } => {
                                        let response = ui
                                            .horizontal(|ui| {
                                                ui.set_height(flat_row_height);

                                                #[allow(clippy::cast_precision_loss)]
                                                let indent_space = *indent as f32;
                                                ui.add_space(indent_space.mul_add(16.0, 8.0));

                                                let icon = if node.is_expanded {
                                                    "⏷ 📁"
                                                } else {
                                                    "⏵ 📁"
                                                };
                                                ui.label(icon);
                                                ui.strong(&node.name);
                                            })
                                            .response;

                                        let row_interact = ui.interact(
                                            response.rect,
                                            response.id.with("dir_row_interact"),
                                            egui::Sense::click(),
                                        );

                                        if row_interact.hovered() {
                                            ui.ctx()
                                                .set_cursor_icon(egui::CursorIcon::PointingHand);
                                            let hover_color = ui
                                                .visuals()
                                                .widgets
                                                .hovered
                                                .bg_fill
                                                .linear_multiply(0.15);
                                            ui.painter().rect_filled(
                                                response.rect.expand(2.0),
                                                egui::CornerRadius::same(4),
                                                hover_color,
                                            );
                                        }

                                        if row_interact.clicked() {
                                            toggle_target = Some(node.path.clone());
                                        }
                                    }
                                    FlatRow::File { node, indent } => {
                                        let response = ui
                                            .horizontal(|ui| {
                                                ui.set_height(flat_row_height);

                                                #[allow(clippy::cast_precision_loss)]
                                                let indent_space = *indent as f32;
                                                ui.add_space(indent_space.mul_add(16.0, 8.0));

                                                let icon = if node.is_expanded {
                                                    "⏷ 📄"
                                                } else {
                                                    "⏵ 📄"
                                                };
                                                ui.label(icon);
                                                ui.strong(&node.name);
                                                let match_count = node.matches.len();
                                                ui.weak(format!("({match_count} matches)"));
                                            })
                                            .response;

                                        let row_interact = ui.interact(
                                            response.rect,
                                            response.id.with("file_row_interact"),
                                            egui::Sense::click(),
                                        );

                                        if row_interact.hovered() {
                                            ui.ctx()
                                                .set_cursor_icon(egui::CursorIcon::PointingHand);
                                            let hover_color = ui
                                                .visuals()
                                                .widgets
                                                .hovered
                                                .bg_fill
                                                .linear_multiply(0.15);
                                            ui.painter().rect_filled(
                                                response.rect.expand(2.0),
                                                egui::CornerRadius::same(4),
                                                hover_color,
                                            );
                                        }

                                        if row_interact.clicked() {
                                            toggle_target = Some(node.path.clone());
                                        }
                                    }
                                    FlatRow::Match {
                                        search_match,
                                        indent,
                                    } => {
                                        let line_number = search_match.line_number;
                                        let line_content = &search_match.line_content;
                                        let path = &search_match.path;

                                        let line_number_str = format!("{line_number:0pad_len$}");

                                        let response = ui
                                            .horizontal(|ui| {
                                                ui.set_height(flat_row_height);

                                                #[allow(clippy::cast_precision_loss)]
                                                let indent_space = *indent as f32;

                                                ui.add_space(
                                                    indent_space.mul_add(16.0, 8.0) + 20.0,
                                                );

                                                let num_text = egui::RichText::new(format!(
                                                    "{line_number_str}:"
                                                ))
                                                .monospace();
                                                ui.colored_label(
                                                    ui.visuals().weak_text_color(),
                                                    num_text,
                                                );

                                                let label_text = if self.monospaced {
                                                    egui::RichText::new(line_content).monospace()
                                                } else {
                                                    egui::RichText::new(line_content)
                                                };
                                                ui.add(
                                                    egui::Label::new(label_text).selectable(true),
                                                );
                                            })
                                            .response;

                                        let row_interact = ui.interact(
                                            response.rect,
                                            response.id.with("match_row_interact"),
                                            egui::Sense::click(),
                                        );

                                        if row_interact.hovered() {
                                            ui.ctx()
                                                .set_cursor_icon(egui::CursorIcon::PointingHand);
                                            let hover_color = ui
                                                .visuals()
                                                .widgets
                                                .hovered
                                                .bg_fill
                                                .linear_multiply(0.15);
                                            ui.painter().rect_filled(
                                                response.rect.expand(2.0),
                                                egui::CornerRadius::same(4),
                                                hover_color,
                                            );
                                        }

                                        if row_interact.clicked() {
                                            open_preview_target =
                                                Some((path.clone(), line_number as usize));
                                        }

                                        row_interact.context_menu(|ui| {
                                            if ui.button("🗁 Open Parent in File Explorer").clicked()
                                            {
                                                open_explorer_target = Some(path.clone());
                                                ui.close();
                                            }
                                        });
                                    }
                                }
                            }
                        });
                    });

                    if let Some(ref target) = toggle_target {
                        self.tree.toggle_expanded(target);
                    }
                } else {
                    let total_rows = self.matches.len();
                    let flat_row_height = ui.spacing().interact_size.y + 4.0;
                    ui.spacing_mut().item_spacing.y = 4.0;

                    scroll_area.show_rows(ui, flat_row_height, total_rows, |ui, row_range| {
                        ui.with_layout(egui::Layout::top_down(egui::Align::LEFT), |ui| {
                            for idx in row_range {
                                let m = &self.matches[idx];
                                let line_number = m.line_number;
                                let line_content = &m.line_content;
                                let path = &m.path;

                                // Pad the line number with leading zeroes
                                let line_number_str = format!("{line_number:0pad_len$}");

                                let response = ui
                                    .horizontal(|ui| {
                                        ui.set_height(flat_row_height);
                                        ui.add_space(8.0); // Margin for text content on the left

                                        // Line numbers are *always* monospaced
                                        let num_text =
                                            egui::RichText::new(format!("{line_number_str}:"))
                                                .monospace();
                                        ui.colored_label(ui.visuals().weak_text_color(), num_text);

                                        // Content label is selectable
                                        let label_text = if self.monospaced {
                                            egui::RichText::new(line_content).monospace()
                                        } else {
                                            egui::RichText::new(line_content)
                                        };
                                        ui.add(egui::Label::new(label_text).selectable(true));

                                        ui.weak(format!("({})", path.display()));
                                    })
                                    .response;

                                // Make the entire row area click / hover responsive
                                let row_interact = ui.interact(
                                    response.rect,
                                    response.id.with("row_interact"),
                                    egui::Sense::click(),
                                );

                                if row_interact.hovered() {
                                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);

                                    let hover_color =
                                        ui.visuals().widgets.hovered.bg_fill.linear_multiply(0.15);
                                    ui.painter().rect_filled(
                                        response.rect.expand(2.0),
                                        egui::CornerRadius::same(4),
                                        hover_color,
                                    );
                                }

                                if row_interact.clicked() {
                                    open_preview_target =
                                        Some((path.clone(), line_number as usize));
                                }

                                row_interact.context_menu(|ui| {
                                    if ui.button("🗁 Open Parent in File Explorer").clicked() {
                                        open_explorer_target = Some(path.clone());
                                        ui.close();
                                    }
                                });
                            }
                        });
                    });
                }

                if let Some((path, line)) = open_preview_target {
                    self.open_preview(path, line);
                }
                if let Some(path) = open_explorer_target {
                    self.open_parent_in_explorer(&path);
                }
            });

        // 5. Interactive File Preview Modal
        if let Some(ref mut previewer) = self.previewer {
            let mut open = true;
            let mut close_preview = false;

            egui::Window::new(egui::WidgetText::RichText(
                egui::RichText::new(format!("Preview: {}", previewer.path.display()))
                    .strong()
                    .into(),
            ))
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .collapsible(false)
            .resizable(true)
            .default_width(700.0)
            .open(&mut open)
            .frame(
                egui::Frame::window(&ui.ctx().global_style())
                    .stroke(egui::Stroke::new(1.5, STROKE_BORDER_SLATE)),
            )
            .show(ui.ctx(), |ui| {
                if let Some(ref err) = previewer.error {
                    ui.colored_label(
                        egui::Color32::from_rgb(239, 68, 68),
                        format!("Error: {err}"),
                    );
                }

                let row_height = ui.spacing().interact_size.y + 4.0;
                let mut load_above = false;
                let mut load_below = false;

                let scroll_height = 400.0;
                let mut scroll_area = egui::ScrollArea::vertical()
                    .max_height(scroll_height)
                    .auto_shrink([false, false]);

                // Programmatically apply mathematical scroll offset on the very first frame to center the target line
                if previewer.needs_scroll_to_target {
                    let target_idx = previewer
                        .target_line
                        .saturating_sub(previewer.loaded_range.start);
                    #[allow(clippy::cast_precision_loss)]
                    let target_y = (target_idx as f32) * row_height;
                    let center_offset = (scroll_height - row_height) / 2.0;
                    let scroll_offset = (target_y - center_offset).max(0.0);
                    scroll_area = scroll_area.scroll_offset(egui::Vec2::new(0.0, scroll_offset));
                    previewer.needs_scroll_to_target = false;
                } else if let Some(added_lines) = previewer.pending_scroll_adjustment.take() {
                    // Instantly shift the scroll offset by exactly the prepended lines height to keep visual state completely stable
                    let new_offset =
                        added_lines.mul_add(row_height, previewer.current_scroll_offset);
                    scroll_area = scroll_area.scroll_offset(egui::Vec2::new(0.0, new_offset));
                }

                // Compute zero-padding width based on max loaded lines count (min 6 digits to eliminate layout shifts)
                let pad_len = previewer.loaded_range.end.to_string().len().max(6);

                let output = scroll_area.show_rows(
                    ui,
                    row_height,
                    previewer.lines.len(),
                    |ui, row_range| {
                        // Check if user scrolled to top or bottom to trigger streaming loads
                        // ONLY trigger streaming loads after we have finished our initial scroll-to-target AND scroll has settled!
                        if !previewer.needs_scroll_to_target && previewer.scroll_settled_delay == 0
                        {
                            if row_range.start == 0 && previewer.loaded_range.start > 1 {
                                load_above = true;
                            }
                            if row_range.end == previewer.lines.len() {
                                load_below = true;
                            }
                        }

                        for idx in row_range {
                            let line_num = previewer.loaded_range.start + idx;
                            let is_target = line_num == previewer.target_line;

                            let num_color = if is_target {
                                COLOR_MATCH_HIGHLIGHT
                            } else {
                                ui.visuals().weak_text_color()
                            };

                            let line_num_str = format!("{line_num:0pad_len$}");

                            ui.horizontal(|ui| {
                                ui.set_height(row_height);
                                // Line numbers are always monospace
                                let num_text =
                                    egui::RichText::new(format!("{line_num_str}:")).monospace();
                                ui.colored_label(num_color, num_text);

                                let line_content = &previewer.lines[idx];
                                let rich_line = if is_target {
                                    if self.monospaced {
                                        egui::RichText::new(line_content).strong().monospace()
                                    } else {
                                        egui::RichText::new(line_content).strong()
                                    }
                                } else {
                                    if self.monospaced {
                                        egui::RichText::new(line_content).monospace()
                                    } else {
                                        egui::RichText::new(line_content)
                                    }
                                };
                                ui.add(egui::Label::new(rich_line));
                            });
                        }
                    },
                );

                // Save current scroll offset for the next frame
                previewer.current_scroll_offset = output.state.offset.y;

                // Decrement the scroll settled delay frame counter and request repaint to animate
                if !previewer.needs_scroll_to_target && previewer.scroll_settled_delay > 0 {
                    previewer.scroll_settled_delay -= 1;
                    ui.ctx().request_repaint();
                }

                if load_above {
                    previewer.load_more_above(40);
                }
                if load_below {
                    previewer.load_more_below(40);
                }

                ui.separator();
                if ui.button("Close").clicked() {
                    close_preview = true;
                }
            });

            if !open || close_preview {
                self.previewer = None;
            }
        }

        // 6. About Description Modal Window
        if self.show_about {
            let mut open_about = true;
            let mut close_about = false;

            egui::Window::new("ℹ About eSearchStat")
                .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                .collapsible(false)
                .resizable(false)
                .open(&mut open_about)
                .show(ui.ctx(), |ui| {
                    ui.vertical_centered(|ui| {
                        ui.heading(
                            egui::RichText::new("🔍 eSearchStat")
                                .strong()
                                .color(ui.visuals().strong_text_color())
                        );
                        ui.label(concat!("v", env!("CARGO_PKG_VERSION")));
                        ui.separator();
                        ui.label("By: Cody Wyatt Neiman (xangelix) <".to_owned() + "neiman" + "@" + "cody.to>");
                        ui.add_space(8.0);
                        ui.label("A high-performance, developer-focused GUI frontend for ripgrep indexing.");
                        ui.add_space(8.0);
                        ui.label("• Left-click on a result for a streaming file previewer modal, centered on your result");
                        ui.label("• Right-click on a result for a context menu to open the parent dir in your File Explorer");
                        ui.label("• Results can be formatted into a collapsible directory tree view");
                        ui.label("• Zero-copy snapshot persistence is powered by bytemuck and private Copy-on-Write memory mapping (mmap)");
                        ui.add_space(10.0);
                        if ui.button("Close").clicked() {
                            close_about = true;
                        }
                    });
                });

            if !open_about || close_about {
                self.show_about = false;
            }
        }
    }
}

fn export_to_text(matches: &[SearchMatch], path: &Path) -> std::io::Result<()> {
    use std::io::Write;
    let mut file = std::fs::File::create(path)?;
    for m in matches {
        writeln!(
            file,
            "{}:{}:{}",
            m.path.display(),
            m.line_number,
            m.line_content
        )?;
    }
    Ok(())
}
