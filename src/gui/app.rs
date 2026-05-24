use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;

use crate::config::Config;
use crate::core::fs_ops::{read_dir, sort_entries, FileEntry, OperationProgress, SortColumn};
use crate::core::theme::{accent_color, apply_theme, is_dark_mode};

use super::editor::EditorState;
use super::file_browser::ContextMenuState;

// ── Tab ─────────────────────────────────────────────────────────────────────

pub struct Tab {
    pub title: String,
    pub path: PathBuf,
    pub history: Vec<PathBuf>,
    pub history_pos: usize,
    pub files: Vec<FileEntry>,
    pub selected: HashSet<PathBuf>,
    pub sort_col: SortColumn,
    pub sort_asc: bool,
    pub error: Option<String>,
}

impl Tab {
    pub fn new(path: PathBuf) -> Self {
        let title = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "/".into());
        let mut tab = Self {
            title,
            path: path.clone(),
            history: vec![path],
            history_pos: 0,
            files: Vec::new(),
            selected: HashSet::new(),
            sort_col: SortColumn::Name,
            sort_asc: true,
            error: None,
        };
        tab.refresh(false);
        tab
    }

    pub fn navigate(&mut self, path: PathBuf, show_hidden: bool) {
        // Truncate forward history
        self.history.truncate(self.history_pos + 1);
        self.history.push(path.clone());
        self.history_pos = self.history.len() - 1;
        self.path = path;
        self.title = self.path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "/".into());
        self.selected.clear();
        self.refresh(show_hidden);
    }

    pub fn go_back(&mut self, show_hidden: bool) -> bool {
        if self.history_pos > 0 {
            self.history_pos -= 1;
            self.path = self.history[self.history_pos].clone();
            self.title = self.path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "/".into());
            self.selected.clear();
            self.refresh(show_hidden);
            true
        } else {
            false
        }
    }

    pub fn go_forward(&mut self, show_hidden: bool) -> bool {
        if self.history_pos + 1 < self.history.len() {
            self.history_pos += 1;
            self.path = self.history[self.history_pos].clone();
            self.title = self.path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "/".into());
            self.selected.clear();
            self.refresh(show_hidden);
            true
        } else {
            false
        }
    }

    pub fn go_up(&mut self, show_hidden: bool) -> bool {
        if let Some(parent) = self.path.parent().map(|p| p.to_path_buf()) {
            self.navigate(parent, show_hidden);
            true
        } else {
            false
        }
    }

    pub fn refresh(&mut self, show_hidden: bool) {
        match read_dir(&self.path, show_hidden) {
            Ok(mut entries) => {
                sort_entries(&mut entries, self.sort_col, self.sort_asc);
                self.files = entries;
                self.error = None;
            }
            Err(e) => {
                self.files.clear();
                self.error = Some(e.to_string());
            }
        }
    }

    pub fn resort(&mut self) {
        sort_entries(&mut self.files, self.sort_col, self.sort_asc);
    }
}

// ── Clipboard ────────────────────────────────────────────────────────────────

#[derive(Default, Clone, PartialEq)]
pub enum ClipOp {
    #[default]
    Copy,
    Cut,
}

#[derive(Default, Clone)]
pub struct Clipboard {
    pub items: Vec<PathBuf>,
    pub op: ClipOp,
}

// ── Background operation ─────────────────────────────────────────────────────

pub struct BgOperation {
    pub description: String,
    pub progress: Arc<OperationProgress>,
}

// ── Rename dialog ─────────────────────────────────────────────────────────────

pub struct RenameDialog {
    pub path: PathBuf,
    pub new_name: String,
}

// ── Delete confirm ────────────────────────────────────────────────────────────

pub struct DeleteConfirm {
    pub paths: Vec<PathBuf>,
}

// ── Main App ─────────────────────────────────────────────────────────────────

pub struct ExploreApp {
    pub config: Config,
    pub tabs: Vec<Tab>,
    pub active_tab: usize,
    pub sidebar_width: f32,
    pub search_query: String,
    pub clipboard: Clipboard,
    pub operations: Vec<BgOperation>,
    pub editor: Option<EditorState>,
    pub context_menu: Option<ContextMenuState>,
    pub rename_dialog: Option<RenameDialog>,
    pub delete_confirm: Option<DeleteConfirm>,
    pub mkdir_dialog: Option<String>,
    pub new_file_dialog: Option<String>,
    pub status_message: Option<(String, std::time::Instant)>,
    pub show_settings: bool,
    pub terminal_list: Vec<String>,
    pub accent: Option<egui::Color32>,
    pub dark_mode: bool,
}

impl ExploreApp {
    pub fn new(cc: &eframe::CreationContext<'_>, start_path: PathBuf) -> Self {
        let config = Config::load();
        let dark_mode = is_dark_mode();
        let accent = accent_color();
        apply_theme(&cc.egui_ctx, dark_mode, accent);

        // Nicer fonts
        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert(
            "inter".to_owned(),
            egui::FontData::from_static(include_bytes!("../../assets/Inter-Regular.ttf")),
        );
        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .insert(0, "inter".to_owned());
        cc.egui_ctx.set_fonts(fonts);

        let terminal_list = crate::core::terminal::detect_terminals();
        let sidebar_width = config.gui.sidebar_width;

        Self {
            config,
            tabs: vec![Tab::new(start_path)],
            active_tab: 0,
            sidebar_width,
            search_query: String::new(),
            clipboard: Clipboard::default(),
            operations: Vec::new(),
            editor: None,
            context_menu: None,
            rename_dialog: None,
            delete_confirm: None,
            mkdir_dialog: None,
            new_file_dialog: None,
            status_message: None,
            show_settings: false,
            terminal_list,
            accent,
            dark_mode,
        }
    }

    pub fn current_tab(&self) -> &Tab {
        &self.tabs[self.active_tab]
    }

    pub fn current_tab_mut(&mut self) -> &mut Tab {
        &mut self.tabs[self.active_tab]
    }

    pub fn set_status(&mut self, msg: impl Into<String>) {
        self.status_message = Some((msg.into(), std::time::Instant::now()));
    }

    pub fn open_editor(&mut self, path: PathBuf) {
        self.editor = Some(EditorState::open(path, &self.config.editor));
    }

    pub fn add_tab(&mut self, path: PathBuf) {
        self.tabs.push(Tab::new(path));
        self.active_tab = self.tabs.len() - 1;
    }

    pub fn close_tab(&mut self, idx: usize) {
        if self.tabs.len() > 1 {
            self.tabs.remove(idx);
            if self.active_tab >= self.tabs.len() {
                self.active_tab = self.tabs.len() - 1;
            }
        }
    }

    /// Paste clipboard items to current directory.
    pub fn paste(&mut self) {
        if self.clipboard.items.is_empty() {
            return;
        }
        let dest = self.current_tab().path.clone();
        let items = self.clipboard.items.clone();
        let total = crate::core::fs_ops::total_size(&items);
        let prog = Arc::new(OperationProgress::new(total));
        let op_prog = prog.clone();

        match self.clipboard.op {
            ClipOp::Copy => {
                crate::core::fs_ops::copy_async(items, dest, prog);
                self.operations.push(BgOperation {
                    description: "Copying…".into(),
                    progress: op_prog,
                });
            }
            ClipOp::Cut => {
                self.clipboard.items.clear();
                crate::core::fs_ops::move_async(items, dest, prog);
                self.operations.push(BgOperation {
                    description: "Moving…".into(),
                    progress: op_prog,
                });
            }
        }
    }
}

impl eframe::App for ExploreApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Clean up finished background operations
        self.operations.retain(|op| !op.progress.is_done());

        // Clear old status messages
        if let Some((_, t)) = &self.status_message {
            if t.elapsed().as_secs() > 4 {
                self.status_message = None;
            }
        }

        // Request repaint if operations running
        if !self.operations.is_empty() {
            ctx.request_repaint();
        }

        // ── Keyboard shortcuts ───────────────────────────────────────────────
        ctx.input(|i| {
            if i.key_pressed(egui::Key::F5) {
                // handled below
            }
        });

        // Render panels
        super::toolbar::show(self, ctx);
        super::sidebar::show(self, ctx);

        // Background operations overlay at bottom of central panel
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            super::toolbar::show_status(self, ui);
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            // Tab bar
            super::file_browser::show_tabs(self, ui);
            // File list
            super::file_browser::show_file_list(self, ui, ctx);
        });

        // Editor window
        if self.editor.is_some() {
            super::editor::show(self, ctx);
        }

        // Dialogs
        show_rename_dialog(self, ctx);
        show_delete_confirm(self, ctx);
        show_mkdir_dialog(self, ctx);
        show_new_file_dialog(self, ctx);
        show_settings(self, ctx);
    }
}

// ── Dialogs ──────────────────────────────────────────────────────────────────

fn show_rename_dialog(app: &mut ExploreApp, ctx: &egui::Context) {
    // Clone data out first to avoid borrow conflicts
    let (path, mut new_name) = match &app.rename_dialog {
        Some(d) => (d.path.clone(), d.new_name.clone()),
        None => return,
    };
    let mut do_rename = false;
    let mut close = false;

    egui::Window::new("Rename")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.label(format!("Rename: {}", path.file_name().unwrap_or_default().to_string_lossy()));
            let response = ui.text_edit_singleline(&mut new_name);
            if response.lost_focus() && ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
                do_rename = true;
            }
            ui.horizontal(|ui| {
                if ui.button("Rename").clicked() { do_rename = true; }
                if ui.button("Cancel").clicked() { close = true; }
            });
        });

    // Write new_name back
    if let Some(ref mut dlg) = app.rename_dialog {
        dlg.new_name = new_name.clone();
    }

    if do_rename {
        let new_path = path.parent().unwrap_or(&path).join(&new_name);
        match crate::core::fs_ops::rename_path(&path, &new_path) {
            Ok(_) => app.set_status(format!("Renamed to {}", new_name)),
            Err(e) => app.set_status(format!("Error: {}", e)),
        }
        app.rename_dialog = None;
        let hidden = app.config.gui.show_hidden;
        app.current_tab_mut().refresh(hidden);
    } else if close {
        app.rename_dialog = None;
    }
}

fn show_delete_confirm(app: &mut ExploreApp, ctx: &egui::Context) {
    let paths = match &app.delete_confirm {
        Some(d) => d.paths.clone(),
        None => return,
    };
    let mut do_delete = false;
    let mut close = false;

    egui::Window::new("⚠ Confirm Delete")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.label(format!("Delete {} item(s)?", paths.len()));
            for p in paths.iter().take(5) {
                ui.label(format!("  • {}", p.file_name().unwrap_or_default().to_string_lossy()));
            }
            if paths.len() > 5 { ui.label("  …and more"); }
            ui.colored_label(egui::Color32::RED, "This cannot be undone.");
            ui.horizontal(|ui| {
                if ui.button("🗑 Delete").clicked() { do_delete = true; }
                if ui.button("Cancel").clicked() { close = true; }
            });
        });

    if do_delete {
        for p in &paths {
            if let Err(e) = crate::core::fs_ops::delete_path(p) {
                app.set_status(format!("Error: {}", e));
            }
        }
        app.set_status(format!("Deleted {} item(s)", paths.len()));
        app.delete_confirm = None;
        let hidden = app.config.gui.show_hidden;
        app.current_tab_mut().refresh(hidden);
    } else if close {
        app.delete_confirm = None;
    }
}

fn show_mkdir_dialog(app: &mut ExploreApp, ctx: &egui::Context) {
    let mut name = match &app.mkdir_dialog {
        Some(n) => n.clone(),
        None => return,
    };
    let mut do_create = false;
    let mut close = false;

    egui::Window::new("New Folder")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.label("Folder name:");
            let r = ui.text_edit_singleline(&mut name);
            if r.lost_focus() && ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
                do_create = true;
            }
            ui.horizontal(|ui| {
                if ui.button("Create").clicked() { do_create = true; }
                if ui.button("Cancel").clicked() { close = true; }
            });
        });

    if let Some(ref mut d) = app.mkdir_dialog { *d = name.clone(); }

    if do_create {
        let cur = app.tabs[app.active_tab].path.clone();
        let new_dir = cur.join(&name);
        match crate::core::fs_ops::create_dir(&new_dir) {
            Ok(_) => app.set_status(format!("Created folder '{}'", name)),
            Err(e) => app.set_status(format!("Error: {}", e)),
        }
        app.mkdir_dialog = None;
        let hidden = app.config.gui.show_hidden;
        app.current_tab_mut().refresh(hidden);
    } else if close {
        app.mkdir_dialog = None;
    }
}

fn show_new_file_dialog(app: &mut ExploreApp, ctx: &egui::Context) {
    let mut name = match &app.new_file_dialog {
        Some(n) => n.clone(),
        None => return,
    };
    let mut do_create = false;
    let mut close = false;

    egui::Window::new("New File")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.label("File name:");
            let r = ui.text_edit_singleline(&mut name);
            if r.lost_focus() && ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
                do_create = true;
            }
            ui.horizontal(|ui| {
                if ui.button("Create").clicked() { do_create = true; }
                if ui.button("Cancel").clicked() { close = true; }
            });
        });

    if let Some(ref mut d) = app.new_file_dialog { *d = name.clone(); }

    if do_create {
        let cur = app.tabs[app.active_tab].path.clone();
        let new_file = cur.join(&name);
        match crate::core::fs_ops::create_file(&new_file) {
            Ok(_) => app.set_status(format!("Created '{}'", name)),
            Err(e) => app.set_status(format!("Error: {}", e)),
        }
        app.new_file_dialog = None;
        let hidden = app.config.gui.show_hidden;
        app.current_tab_mut().refresh(hidden);
    } else if close {
        app.new_file_dialog = None;
    }
}

fn show_settings(app: &mut ExploreApp, ctx: &egui::Context) {
    if !app.show_settings { return; }

    let terminal_list = app.terminal_list.clone();
    let mut close_settings = false;
    let mut save_settings = false;

    egui::Window::new("⚙ Settings")
        .collapsible(false)
        .resizable(true)
        .min_width(400.0)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.heading("Terminal");
                ui.horizontal(|ui| {
                    ui.label("Preferred terminal:");
                    egui::ComboBox::from_id_source("terminal_combo")
                        .selected_text(&app.config.terminal.preferred)
                        .show_ui(ui, |ui| {
                            let mut opts = vec!["auto".to_string(), "custom".to_string()];
                            opts.extend(terminal_list.clone());
                            for opt in opts {
                                let label = opt.clone();
                                ui.selectable_value(&mut app.config.terminal.preferred, opt, label);
                            }
                        });
                });
                if app.config.terminal.preferred == "custom" {
                    ui.horizontal(|ui| {
                        ui.label("Command:");
                        ui.text_edit_singleline(&mut app.config.terminal.custom_command);
                    });
                    ui.horizontal(|ui| {
                        ui.label("Args (use {path}):");
                        ui.text_edit_singleline(&mut app.config.terminal.custom_args);
                    });
                }

                ui.separator();
                ui.heading("Editor");
                ui.checkbox(&mut app.config.editor.syntax_highlight, "Syntax highlighting");
                ui.checkbox(&mut app.config.editor.word_wrap, "Word wrap");
                ui.checkbox(&mut app.config.editor.show_line_numbers, "Line numbers");
                ui.horizontal(|ui| {
                    ui.label("Tab size:");
                    ui.add(egui::DragValue::new(&mut app.config.editor.tab_size).range(1..=8));
                });
                ui.horizontal(|ui| {
                    ui.label("Font size:");
                    ui.add(egui::DragValue::new(&mut app.config.editor.font_size).range(8.0..=32.0));
                });

                ui.separator();
                ui.heading("Files");
                ui.checkbox(&mut app.config.gui.show_hidden, "Show hidden files");
                ui.checkbox(&mut app.config.gui.confirm_delete, "Confirm before delete");

                ui.separator();
                ui.heading("Bookmarks");
                let mut to_remove = None;
                for (i, bm) in app.config.bookmarks.iter().enumerate() {
                    ui.horizontal(|ui| {
                        ui.label(bm.to_string_lossy().as_ref());
                        if ui.small_button("✖").clicked() {
                            to_remove = Some(i);
                        }
                    });
                }
                if let Some(i) = to_remove {
                    app.config.bookmarks.remove(i);
                }

                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("💾 Save").clicked() { save_settings = true; }
                    if ui.button("Close").clicked() { close_settings = true; }
                });
            });
        });

    if save_settings {
        let _ = app.config.save();
        app.set_status("Settings saved.");
    }
    if close_settings {
        app.show_settings = false;
    }
}
