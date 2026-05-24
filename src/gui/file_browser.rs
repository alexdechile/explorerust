use egui::Context;
use egui_extras::{Column, TableBuilder};
use std::path::PathBuf;

use crate::core::fs_ops::SortColumn;
use super::app::{ClipOp, Clipboard, DeleteConfirm, ExploreApp, RenameDialog};

// ── Context menu state ────────────────────────────────────────────────────────

pub struct ContextMenuState {
    pub path: PathBuf,
    pub is_dir: bool,
}

// ── Tab bar ───────────────────────────────────────────────────────────────────

pub fn show_tabs(app: &mut ExploreApp, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        let mut close_tab: Option<usize> = None;

        for i in 0..app.tabs.len() {
            let active = i == app.active_tab;
            let title = app.tabs[i].title.clone();
            let label = if active {
                egui::RichText::new(&title).strong()
            } else {
                egui::RichText::new(&title)
            };

            ui.horizontal(|ui| {
                if ui.selectable_label(active, label).clicked() {
                    app.active_tab = i;
                }
                if app.tabs.len() > 1 && ui.small_button("✖").clicked() {
                    close_tab = Some(i);
                }
            });

            if i < app.tabs.len() - 1 {
                ui.separator();
            }
        }

        if let Some(idx) = close_tab {
            app.close_tab(idx);
        }
    });
    ui.separator();
}

// ── File list ─────────────────────────────────────────────────────────────────

pub fn show_file_list(app: &mut ExploreApp, ui: &mut egui::Ui, ctx: &Context) {
    // Error banner
    if let Some(ref err) = app.tabs[app.active_tab].error.clone() {
        ui.colored_label(egui::Color32::RED, format!("⚠ {}", err));
        return;
    }

    // Handle keyboard shortcuts in the file list
    handle_keyboard(app, ctx);

    // Filter by search
    let query = app.search_query.to_lowercase();

    let show_hidden = app.config.gui.show_hidden;
    let files: Vec<_> = app.tabs[app.active_tab].files
        .iter()
        .filter(|f| query.is_empty() || f.name.to_lowercase().contains(&query))
        .cloned()
        .collect();

    egui::ScrollArea::vertical()
        .auto_shrink([false; 2])
        .show(ui, |ui| {
            let table = TableBuilder::new(ui)
                .striped(true)
                .resizable(true)
                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                .column(Column::initial(300.0).at_least(100.0))  // Name
                .column(Column::initial(80.0).at_least(50.0))    // Size
                .column(Column::initial(80.0).at_least(60.0))    // Type
                .column(Column::initial(140.0).at_least(100.0))  // Modified
                .column(Column::initial(90.0).at_least(80.0))    // Permissions
                .column(Column::remainder());                     // Owner

            let sort_col = app.tabs[app.active_tab].sort_col;
            let sort_asc = app.tabs[app.active_tab].sort_asc;

            table
                .header(22.0, |mut header| {
                    sort_header(&mut header, app, SortColumn::Name, sort_col, sort_asc, "Name");
                    sort_header(&mut header, app, SortColumn::Size, sort_col, sort_asc, "Size");
                    sort_header(&mut header, app, SortColumn::Kind, sort_col, sort_asc, "Type");
                    sort_header(&mut header, app, SortColumn::Modified, sort_col, sort_asc, "Modified");
                    sort_header(&mut header, app, SortColumn::Permissions, sort_col, sort_asc, "Permissions");
                    header.col(|ui| { ui.strong("Owner"); });
                })
                .body(|body| {
                    body.rows(20.0, files.len(), |mut row| {
                        let f = &files[row.index()];
                        let is_selected = app.tabs[app.active_tab].selected.contains(&f.path);
                        let fg = if is_selected {
                            Some(egui::Color32::WHITE)
                        } else if f.is_dir {
                            Some(egui::Color32::from_rgb(100, 170, 255))
                        } else if f.is_symlink {
                            Some(egui::Color32::from_rgb(100, 220, 180))
                        } else {
                            None
                        };

                        row.set_selected(is_selected);

                        // Name column (icon + name)
                        row.col(|ui| {
                            let icon = file_icon(f);
                            let text = format!("{} {}", icon, f.name);
                            let label = if let Some(color) = fg {
                                egui::RichText::new(&text).color(color)
                            } else {
                                egui::RichText::new(&text)
                            };

                            let resp = ui.selectable_label(is_selected, label);

                            if resp.double_clicked() {
                                handle_open(app, f.path.clone(), f.is_dir, show_hidden);
                            }

                            if resp.clicked() {
                                let path = f.path.clone();
                                let is_multi = ui.input(|i| i.modifiers.ctrl);
                                let is_range = ui.input(|i| i.modifiers.shift);
                                if is_multi {
                                    if app.tabs[app.active_tab].selected.contains(&path) {
                                        app.tabs[app.active_tab].selected.remove(&path);
                                    } else {
                                        app.tabs[app.active_tab].selected.insert(path);
                                    }
                                } else if !is_range {
                                    app.tabs[app.active_tab].selected.clear();
                                    app.tabs[app.active_tab].selected.insert(path);
                                }
                            }

                            // Context menu on right-click
                            resp.context_menu(|ui| {
                                show_context_menu(app, ui, f.path.clone(), f.is_dir);
                            });
                        });

                        row.col(|ui| { ui.label(f.size_display()); });
                        row.col(|ui| { ui.label(f.kind_display()); });
                        row.col(|ui| { ui.label(f.date_display()); });
                        row.col(|ui| {
                            ui.label(egui::RichText::new(&f.permissions).monospace().small());
                        });
                        row.col(|ui| { ui.label(&f.owner); });
                    });
                });
        });
}

fn sort_header(
    header: &mut egui_extras::TableRow,
    app: &mut ExploreApp,
    col: SortColumn,
    current: SortColumn,
    asc: bool,
    label: &str,
) {
    header.col(|ui| {
        let arrow = if current == col {
            if asc { " ▲" } else { " ▼" }
        } else {
            ""
        };
        if ui.button(format!("{}{}", label, arrow)).clicked() {
            let tab = &mut app.tabs[app.active_tab];
            if tab.sort_col == col {
                tab.sort_asc = !tab.sort_asc;
            } else {
                tab.sort_col = col;
                tab.sort_asc = true;
            }
            tab.resort();
        }
    });
}

fn file_icon(f: &crate::core::fs_ops::FileEntry) -> &'static str {
    if f.is_dir {
        return "📁";
    }
    if f.is_symlink {
        return "🔗";
    }
    match f.extension.to_lowercase().as_str() {
        "rs" => "🦀",
        "toml" | "yaml" | "yml" | "json" | "ron" => "⚙",
        "md" | "markdown" => "📝",
        "txt" => "📄",
        "png" | "jpg" | "jpeg" | "gif" | "svg" | "webp" | "bmp" => "🖼",
        "mp4" | "mkv" | "avi" | "mov" | "webm" => "🎬",
        "mp3" | "flac" | "ogg" | "wav" | "aac" => "🎵",
        "zip" | "tar" | "gz" | "xz" | "bz2" | "zst" | "7z" | "rar" => "📦",
        "pdf" => "📕",
        "sh" | "bash" | "zsh" | "fish" => "📜",
        "py" => "🐍",
        "js" | "ts" => "📜",
        "html" | "htm" => "🌐",
        "css" => "🎨",
        "exe" | "bin" | "elf" => "⚙",
        "deb" | "rpm" | "pkg" => "📦",
        _ => "📄",
    }
}

fn handle_open(app: &mut ExploreApp, path: PathBuf, is_dir: bool, show_hidden: bool) {
    if is_dir {
        app.tabs[app.active_tab].navigate(path, show_hidden);
    } else {
        // Text files: open in editor
        let ext = path.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
        if matches!(ext.as_str(), "txt" | "md" | "markdown" | "rs" | "toml" | "yaml" | "yml"
            | "json" | "sh" | "bash" | "zsh" | "py" | "js" | "ts" | "html" | "css"
            | "ron" | "cfg" | "conf" | "ini" | "log" | "csv" | "") {
            app.open_editor(path);
        } else {
            // Open with system default
            if let Err(e) = open::that(&path) {
                app.set_status(format!("Cannot open: {}", e));
            }
        }
    }
}

fn show_context_menu(
    app: &mut ExploreApp,
    ui: &mut egui::Ui,
    path: PathBuf,
    is_dir: bool,
) {
    let hidden = app.config.gui.show_hidden;

    if is_dir {
        if ui.button("📂 Open in new tab").clicked() {
            app.add_tab(path.clone());
            ui.close_menu();
        }
        if ui.button("⬛ Open terminal here").clicked() {
            let cfg = &app.config.terminal;
            if let Err(e) = crate::core::terminal::open_terminal(
                &cfg.preferred, &cfg.custom_command, &cfg.custom_args, &path
            ) {
                app.set_status(format!("Terminal error: {}", e));
            }
            ui.close_menu();
        }
    } else {
        if ui.button("✏ Edit").clicked() {
            app.open_editor(path.clone());
            ui.close_menu();
        }
        if ui.button("🔗 Open with default app").clicked() {
            let _ = open::that(&path);
            ui.close_menu();
        }
    }

    ui.separator();

    if ui.button("📋 Copy").clicked() {
        let selected = get_selection(app, &path);
        app.clipboard = Clipboard { items: selected, op: ClipOp::Copy };
        ui.close_menu();
    }
    if ui.button("✂ Cut").clicked() {
        let selected = get_selection(app, &path);
        app.clipboard = Clipboard { items: selected, op: ClipOp::Cut };
        ui.close_menu();
    }
    if !app.clipboard.items.is_empty() {
        if ui.button("📌 Paste").clicked() {
            app.paste();
            ui.close_menu();
        }
    }

    ui.separator();

    if ui.button("✏ Rename").clicked() {
        app.rename_dialog = Some(RenameDialog {
            path: path.clone(),
            new_name: path.file_name().unwrap_or_default().to_string_lossy().to_string(),
        });
        ui.close_menu();
    }

    if ui.button("🔖 Bookmark this folder").clicked() {
        let bm = if is_dir { path.clone() } else { path.parent().unwrap_or(&path).to_path_buf() };
        if !app.config.bookmarks.contains(&bm) {
            app.config.bookmarks.push(bm);
            let _ = app.config.save();
        }
        ui.close_menu();
    }

    ui.separator();

    if ui.button("🗑 Delete").clicked() {
        let selected = get_selection(app, &path);
        if app.config.gui.confirm_delete {
            app.delete_confirm = Some(DeleteConfirm { paths: selected });
        } else {
            for p in &selected {
                let _ = crate::core::fs_ops::delete_path(p);
            }
            app.current_tab_mut().refresh(hidden);
        }
        ui.close_menu();
    }
}

fn get_selection(app: &ExploreApp, fallback: &PathBuf) -> Vec<PathBuf> {
    let selected = &app.tabs[app.active_tab].selected;
    if selected.is_empty() {
        vec![fallback.clone()]
    } else {
        selected.iter().cloned().collect()
    }
}

fn handle_keyboard(app: &mut ExploreApp, ctx: &Context) {
    let hidden = app.config.gui.show_hidden;
    ctx.input(|i| {
        // F5 refresh
        if i.key_pressed(egui::Key::F5) {
            app.tabs[app.active_tab].refresh(hidden);
        }
        // Alt+Left = back
        if i.key_pressed(egui::Key::ArrowLeft) && i.modifiers.alt {
            app.current_tab_mut().go_back(hidden);
        }
        // Alt+Right = forward
        if i.key_pressed(egui::Key::ArrowRight) && i.modifiers.alt {
            app.current_tab_mut().go_forward(hidden);
        }
        // Alt+Up = parent
        if i.key_pressed(egui::Key::ArrowUp) && i.modifiers.alt {
            app.current_tab_mut().go_up(hidden);
        }
        // Ctrl+C copy
        if i.key_pressed(egui::Key::C) && i.modifiers.ctrl {
            let selected: Vec<_> = app.tabs[app.active_tab].selected.iter().cloned().collect();
            if !selected.is_empty() {
                app.clipboard = Clipboard { items: selected, op: ClipOp::Copy };
            }
        }
        // Ctrl+X cut
        if i.key_pressed(egui::Key::X) && i.modifiers.ctrl {
            let selected: Vec<_> = app.tabs[app.active_tab].selected.iter().cloned().collect();
            if !selected.is_empty() {
                app.clipboard = Clipboard { items: selected, op: ClipOp::Cut };
            }
        }
        // Ctrl+V paste
        if i.key_pressed(egui::Key::V) && i.modifiers.ctrl {
            app.paste();
        }
        // Delete key
        if i.key_pressed(egui::Key::Delete) {
            let selected: Vec<_> = app.tabs[app.active_tab].selected.iter().cloned().collect();
            if !selected.is_empty() {
                app.delete_confirm = Some(DeleteConfirm { paths: selected });
            }
        }
        // Ctrl+T new tab
        if i.key_pressed(egui::Key::T) && i.modifiers.ctrl {
            let p = app.current_tab().path.clone();
            app.add_tab(p);
        }
        // Ctrl+W close tab
        if i.key_pressed(egui::Key::W) && i.modifiers.ctrl {
            let idx = app.active_tab;
            app.close_tab(idx);
        }
    });
}
