use egui::Context;
use std::path::PathBuf;

use super::app::ExploreApp;

pub fn show(app: &mut ExploreApp, ctx: &Context) {
    egui::SidePanel::left("sidebar")
        .resizable(true)
        .min_width(140.0)
        .max_width(320.0)
        .default_width(app.config.gui.sidebar_width)
        .show(ctx, |ui| {
            app.config.gui.sidebar_width = ui.available_width() + ui.spacing().item_spacing.x;
            egui::ScrollArea::vertical().show(ui, |ui| {
                // ── Quick access ──────────────────────────────────────────
                ui.add_space(4.0);
                section_label(ui, "Quick Access");

                quick_link(ui, app, "🏠 Home", dirs::home_dir());
                quick_link(ui, app, "🖥 Desktop", dirs::desktop_dir());
                quick_link(ui, app, "📄 Documents", dirs::document_dir());
                quick_link(ui, app, "⬇ Downloads", dirs::download_dir());
                quick_link(ui, app, "🖼 Pictures", dirs::picture_dir());
                quick_link(ui, app, "🎵 Music", dirs::audio_dir());
                quick_link(ui, app, "🎬 Videos", dirs::video_dir());

                ui.separator();

                // ── Bookmarks ─────────────────────────────────────────────
                section_label(ui, "Bookmarks");

                let bookmarks = app.config.bookmarks.clone();
                let current = app.current_tab().path.clone();
                let hidden = app.config.gui.show_hidden;

                let mut add_bookmark = false;
                for bm in &bookmarks {
                    let name = bm.file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| bm.to_string_lossy().to_string());
                    let active = *bm == current;
                    let resp = ui.selectable_label(active, format!("🔖 {}", name))
                        .on_hover_text(bm.to_string_lossy().as_ref());
                    if resp.clicked() {
                        app.tabs[app.active_tab].navigate(bm.clone(), hidden);
                    }
                    resp.context_menu(|ui| {
                        if ui.button("Remove bookmark").clicked() {
                            app.config.bookmarks.retain(|b| b != bm);
                            ui.close_menu();
                        }
                    });
                }

                // Add current folder as bookmark
                ui.add_space(2.0);
                if ui.small_button("＋ Bookmark current").clicked() {
                    add_bookmark = true;
                }
                if add_bookmark && !app.config.bookmarks.contains(&current) {
                    app.config.bookmarks.push(current);
                    let _ = app.config.save();
                }

                ui.separator();

                // ── Filesystem roots / mounts ─────────────────────────────
                section_label(ui, "Devices");

                // Always show root
                nav_button(ui, app, "💽 / (root)", &PathBuf::from("/"));

                // Read /proc/mounts to list mounted filesystems
                if let Ok(mounts) = std::fs::read_to_string("/proc/mounts") {
                    for line in mounts.lines() {
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        if parts.len() < 2 { continue; }
                        let mountpoint = parts[1];
                        // Show only /media, /mnt, /run/media entries (removable drives)
                        if mountpoint.starts_with("/media")
                            || mountpoint.starts_with("/mnt")
                            || mountpoint.starts_with("/run/media")
                        {
                            let label = mountpoint.split('/').last().unwrap_or(mountpoint);
                            nav_button(ui, app, &format!("💾 {}", label), &PathBuf::from(mountpoint));
                        }
                    }
                }

                ui.add_space(4.0);
            });
        });
}

fn section_label(ui: &mut egui::Ui, label: &str) {
    ui.add_space(4.0);
    ui.label(egui::RichText::new(label).small().weak());
    ui.add_space(2.0);
}

fn quick_link(ui: &mut egui::Ui, app: &mut ExploreApp, label: &str, dir: Option<PathBuf>) {
    if let Some(p) = dir {
        if p.exists() {
            nav_button(ui, app, label, &p);
        }
    }
}

fn nav_button(ui: &mut egui::Ui, app: &mut ExploreApp, label: &str, path: &PathBuf) {
    let current = &app.tabs[app.active_tab].path;
    let active = current == path;
    let hidden = app.config.gui.show_hidden;
    if ui.selectable_label(active, label).clicked() {
        app.tabs[app.active_tab].navigate(path.clone(), hidden);
    }
}
