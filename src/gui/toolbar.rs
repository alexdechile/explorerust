use egui::Context;

use super::app::ExploreApp;

pub fn show(app: &mut ExploreApp, ctx: &Context) {
    egui::TopBottomPanel::top("toolbar").min_height(42.0).show(ctx, |ui| {
        ui.horizontal_centered(|ui| {
            ui.add_space(4.0);

            // Nav buttons
            let tab = &app.tabs[app.active_tab];
            let can_back = tab.history_pos > 0;
            let can_fwd = tab.history_pos + 1 < tab.history.len();
            let can_up = tab.path.parent().is_some();

            if ui.add_enabled(can_back, egui::Button::new("◀")).on_hover_text("Back (Alt+Left)").clicked() {
                let hidden = app.config.gui.show_hidden;
                app.current_tab_mut().go_back(hidden);
            }
            if ui.add_enabled(can_fwd, egui::Button::new("▶")).on_hover_text("Forward (Alt+Right)").clicked() {
                let hidden = app.config.gui.show_hidden;
                app.current_tab_mut().go_forward(hidden);
            }
            if ui.add_enabled(can_up, egui::Button::new("▲")).on_hover_text("Up (Alt+Up)").clicked() {
                let hidden = app.config.gui.show_hidden;
                app.current_tab_mut().go_up(hidden);
            }
            if ui.button("↺").on_hover_text("Refresh (F5)").clicked() {
                let hidden = app.config.gui.show_hidden;
                app.current_tab_mut().refresh(hidden);
            }

            ui.separator();

            // Breadcrumb
            let current_path = app.current_tab().path.clone();
            let components: Vec<_> = current_path.components().collect();
            let mut build = std::path::PathBuf::new();
            for (i, comp) in components.iter().enumerate() {
                build.push(comp);
                let label = if i == 0 {
                    "/".to_string()
                } else {
                    comp.as_os_str().to_string_lossy().to_string()
                };
                if ui.selectable_label(false, &label).clicked() {
                    let p = build.clone();
                    let hidden = app.config.gui.show_hidden;
                    app.current_tab_mut().navigate(p, hidden);
                }
                if i < components.len() - 1 {
                    ui.label("›");
                }
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Settings button
                if ui.button("⚙").on_hover_text("Settings").clicked() {
                    app.show_settings = true;
                }

                // Show hidden toggle
                let hidden_label = if app.config.gui.show_hidden { "🙈" } else { "👁" };
                if ui.button(hidden_label).on_hover_text("Toggle hidden files").clicked() {
                    app.config.gui.show_hidden = !app.config.gui.show_hidden;
                    let hidden = app.config.gui.show_hidden;
                    app.current_tab_mut().refresh(hidden);
                }

                // Open terminal
                if ui.button("⬛ Terminal").on_hover_text("Open terminal here").clicked() {
                    let dir = app.current_tab().path.clone();
                    let cfg = &app.config.terminal;
                    if let Err(e) = crate::core::terminal::open_terminal(
                        &cfg.preferred,
                        &cfg.custom_command,
                        &cfg.custom_args,
                        &dir,
                    ) {
                        app.set_status(format!("Terminal error: {}", e));
                    }
                }

                // New file / folder
                if ui.button("📄+").on_hover_text("New file").clicked() {
                    app.new_file_dialog = Some(String::new());
                }
                if ui.button("📁+").on_hover_text("New folder").clicked() {
                    app.mkdir_dialog = Some(String::new());
                }

                // Search bar
                ui.add_space(8.0);
                let search = egui::TextEdit::singleline(&mut app.search_query)
                    .hint_text("🔍 Search…")
                    .desired_width(180.0);
                ui.add(search);

                // New tab
                if ui.button("＋").on_hover_text("New tab").clicked() {
                    let path = app.current_tab().path.clone();
                    app.add_tab(path);
                }
            });
        });
    });
}

pub fn show_status(app: &mut ExploreApp, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        // Selected / total items
        let tab = &app.tabs[app.active_tab];
        let total = tab.files.len();
        let selected = tab.selected.len();
        if selected > 0 {
            ui.label(format!("{} selected / {} items", selected, total));
        } else {
            ui.label(format!("{} items", total));
        }

        ui.separator();
        ui.label(tab.path.to_string_lossy().as_ref());

        // Background operations
        for op in &app.operations {
            ui.separator();
            ui.label(&op.description);
            let frac = op.progress.fraction();
            ui.add(egui::ProgressBar::new(frac).desired_width(120.0));
        }

        // Status message
        if let Some((msg, _)) = &app.status_message {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(msg);
            });
        }
    });
}
