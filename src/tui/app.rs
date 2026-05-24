use std::collections::HashSet;
use std::path::PathBuf;

use crate::config::Config;
use crate::core::fs_ops::{read_dir, sort_entries, FileEntry, SortColumn};

#[derive(PartialEq, Clone, Copy)]
pub enum TuiMode {
    Browse,
    Search,
    Rename,
    Mkdir,
    NewFile,
    Delete,
    Editor,
    Help,
}

#[derive(Clone)]
pub struct EditorContent {
    pub path: PathBuf,
    pub lines: Vec<String>,
    pub cursor_row: usize,
    pub cursor_col: usize,
    pub modified: bool,
    pub scroll_offset: usize,
}

impl EditorContent {
    pub fn open(path: PathBuf) -> Self {
        let content = std::fs::read_to_string(&path).unwrap_or_default();
        let lines: Vec<String> = if content.is_empty() {
            vec![String::new()]
        } else {
            content.lines().map(|l| l.to_string()).collect()
        };
        Self { path, lines, cursor_row: 0, cursor_col: 0, modified: false, scroll_offset: 0 }
    }

    pub fn save(&mut self) -> Result<(), String> {
        let content = self.lines.join("\n");
        std::fs::write(&self.path, content).map_err(|e| e.to_string())?;
        self.modified = false;
        Ok(())
    }

    pub fn current_line(&self) -> &str {
        self.lines.get(self.cursor_row).map(|s| s.as_str()).unwrap_or("")
    }

    pub fn insert_char(&mut self, c: char) {
        let row = self.cursor_row;
        let col = self.cursor_col;
        if row < self.lines.len() {
            self.lines[row].insert(col, c);
            self.cursor_col += 1;
            self.modified = true;
        }
    }

    pub fn backspace(&mut self) {
        let row = self.cursor_row;
        let col = self.cursor_col;
        if col > 0 {
            self.lines[row].remove(col - 1);
            self.cursor_col -= 1;
            self.modified = true;
        } else if row > 0 {
            let line = self.lines.remove(row);
            self.cursor_row -= 1;
            self.cursor_col = self.lines[self.cursor_row].len();
            self.lines[self.cursor_row].push_str(&line);
            self.modified = true;
        }
    }

    pub fn newline(&mut self) {
        let row = self.cursor_row;
        let col = self.cursor_col;
        let rest = self.lines[row].split_off(col);
        self.lines.insert(row + 1, rest);
        self.cursor_row += 1;
        self.cursor_col = 0;
        self.modified = true;
    }
}

pub struct TuiApp {
    pub config: Config,
    pub path: PathBuf,
    pub files: Vec<FileEntry>,
    pub cursor: usize,
    pub scroll_offset: usize,
    pub selected: HashSet<PathBuf>,
    pub sort_col: SortColumn,
    pub sort_asc: bool,
    pub mode: TuiMode,
    pub search_query: String,
    pub input_buf: String,
    pub status: String,
    pub clipboard: Vec<PathBuf>,
    pub clipboard_cut: bool,
    pub editor: Option<EditorContent>,
    pub needs_refresh: bool,
    pub history: Vec<PathBuf>,
    pub history_pos: usize,
    pub available_terminals: Vec<String>,
}

impl TuiApp {
    pub fn new(start_path: PathBuf) -> Self {
        let config = Config::load();
        let available_terminals = crate::core::terminal::detect_terminals();
        let mut app = Self {
            config,
            path: start_path.clone(),
            files: Vec::new(),
            cursor: 0,
            scroll_offset: 0,
            selected: HashSet::new(),
            sort_col: SortColumn::Name,
            sort_asc: true,
            mode: TuiMode::Browse,
            search_query: String::new(),
            input_buf: String::new(),
            status: String::new(),
            clipboard: Vec::new(),
            clipboard_cut: false,
            editor: None,
            needs_refresh: false,
            history: vec![start_path],
            history_pos: 0,
            available_terminals,
        };
        app.refresh();
        app
    }

    pub fn refresh(&mut self) {
        match read_dir(&self.path, self.config.gui.show_hidden) {
            Ok(mut entries) => {
                sort_entries(&mut entries, self.sort_col, self.sort_asc);
                self.files = entries;
                if self.cursor >= self.files.len() && !self.files.is_empty() {
                    self.cursor = self.files.len() - 1;
                }
            }
            Err(e) => {
                self.set_status(format!("Error: {}", e));
                self.files.clear();
            }
        }
    }

    pub fn navigate_to(&mut self, path: PathBuf) {
        self.history.truncate(self.history_pos + 1);
        self.history.push(path.clone());
        self.history_pos = self.history.len() - 1;
        self.path = path;
        self.cursor = 0;
        self.scroll_offset = 0;
        self.selected.clear();
        self.refresh();
    }

    pub fn go_back(&mut self) {
        if self.history_pos > 0 {
            self.history_pos -= 1;
            self.path = self.history[self.history_pos].clone();
            self.cursor = 0;
            self.scroll_offset = 0;
            self.refresh();
        }
    }

    pub fn go_forward(&mut self) {
        if self.history_pos + 1 < self.history.len() {
            self.history_pos += 1;
            self.path = self.history[self.history_pos].clone();
            self.cursor = 0;
            self.scroll_offset = 0;
            self.refresh();
        }
    }

    pub fn go_up(&mut self) {
        if let Some(parent) = self.path.parent().map(|p| p.to_path_buf()) {
            self.navigate_to(parent);
        }
    }

    pub fn cursor_file(&self) -> Option<&FileEntry> {
        if self.mode == TuiMode::Search {
            let query = self.search_query.to_lowercase();
            let filtered: Vec<&FileEntry> = self.files.iter()
                .filter(|f| f.name.to_lowercase().contains(&query))
                .collect();
            filtered.get(self.cursor).copied()
        } else {
            self.files.get(self.cursor)
        }
    }

    pub fn filtered_files(&self) -> Vec<&FileEntry> {
        if self.mode == TuiMode::Search && !self.search_query.is_empty() {
            let q = self.search_query.to_lowercase();
            self.files.iter().filter(|f| f.name.to_lowercase().contains(&q)).collect()
        } else {
            self.files.iter().collect()
        }
    }

    pub fn set_status(&mut self, msg: impl Into<String>) {
        self.status = msg.into();
    }

    pub fn open_terminal(&mut self) {
        let cfg = &self.config.terminal;
        if let Err(e) = crate::core::terminal::open_terminal(
            &cfg.preferred, &cfg.custom_command, &cfg.custom_args, &self.path
        ) {
            self.set_status(format!("Terminal error: {}", e));
        } else {
            self.set_status("Terminal opened.");
        }
    }

    pub fn scroll_cursor(&mut self, height: usize) {
        if self.cursor < self.scroll_offset {
            self.scroll_offset = self.cursor;
        }
        if self.cursor >= self.scroll_offset + height {
            self.scroll_offset = self.cursor - height + 1;
        }
    }
}
