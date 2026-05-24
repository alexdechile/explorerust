use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::app::{EditorContent, TuiApp, TuiMode};
use crate::core::fs_ops::{delete_path, rename_path, create_dir, create_file, SortColumn};

/// Returns true if the app should quit.
pub fn handle(app: &mut TuiApp, key: KeyEvent) -> bool {
    match app.mode {
        TuiMode::Editor => handle_editor(app, key),
        TuiMode::Search => handle_search(app, key),
        TuiMode::Rename => handle_rename(app, key),
        TuiMode::Mkdir => handle_mkdir(app, key),
        TuiMode::NewFile => handle_new_file(app, key),
        TuiMode::Delete => handle_delete(app, key),
        TuiMode::Help => {
            app.mode = TuiMode::Browse;
            false
        }
        TuiMode::Browse => handle_browse(app, key),
    }
}

fn handle_browse(app: &mut TuiApp, key: KeyEvent) -> bool {
    let hidden = app.config.gui.show_hidden;
    let files = app.filtered_files().len();

    match key.code {
        // Quit
        KeyCode::Char('q') | KeyCode::Char('Q') => return true,
        KeyCode::Esc => return true,

        // Navigation
        KeyCode::Up | KeyCode::Char('k') => {
            if app.cursor > 0 { app.cursor -= 1; }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if app.cursor + 1 < files { app.cursor += 1; }
        }
        KeyCode::PageUp => {
            app.cursor = app.cursor.saturating_sub(10);
        }
        KeyCode::PageDown => {
            app.cursor = (app.cursor + 10).min(files.saturating_sub(1));
        }
        KeyCode::Home => { app.cursor = 0; }
        KeyCode::End => { app.cursor = files.saturating_sub(1); }

        // Open / enter
        KeyCode::Enter | KeyCode::Char('l') | KeyCode::Right => {
            if let Some(f) = app.cursor_file().cloned() {
                if f.is_dir {
                    app.navigate_to(f.path);
                } else {
                    app.editor = Some(EditorContent::open(f.path));
                    app.mode = TuiMode::Editor;
                }
            }
        }

        // Go up / back
        KeyCode::Backspace | KeyCode::Char('h') => {
            app.go_up();
        }
        KeyCode::Left if key.modifiers.contains(KeyModifiers::ALT) => {
            app.go_back();
        }
        KeyCode::Left => {
            app.go_up();
        }
        KeyCode::Right if key.modifiers.contains(KeyModifiers::ALT) => {
            app.go_forward();
        }

        // Select
        KeyCode::Char(' ') => {
            if let Some(f) = app.cursor_file().cloned() {
                if app.selected.contains(&f.path) {
                    app.selected.remove(&f.path);
                } else {
                    app.selected.insert(f.path);
                }
                if app.cursor + 1 < files { app.cursor += 1; }
            }
        }

        // Open in editor
        KeyCode::Char('e') => {
            if let Some(f) = app.cursor_file().cloned() {
                if !f.is_dir {
                    app.editor = Some(EditorContent::open(f.path));
                    app.mode = TuiMode::Editor;
                }
            }
        }

        // Open terminal
        KeyCode::Char('t') => {
            app.open_terminal();
        }

        // Copy
        KeyCode::Char('c') => {
            if !app.selected.is_empty() {
                app.clipboard = app.selected.iter().cloned().collect();
                app.clipboard_cut = false;
                app.set_status(format!("Copied {} item(s)", app.clipboard.len()));
            } else if let Some(f) = app.cursor_file().cloned() {
                app.clipboard = vec![f.path];
                app.clipboard_cut = false;
                app.set_status("Copied 1 item");
            }
        }

        // Cut
        KeyCode::Char('x') => {
            if !app.selected.is_empty() {
                app.clipboard = app.selected.iter().cloned().collect();
                app.clipboard_cut = true;
                app.set_status(format!("Cut {} item(s)", app.clipboard.len()));
            } else if let Some(f) = app.cursor_file().cloned() {
                app.clipboard = vec![f.path];
                app.clipboard_cut = true;
                app.set_status("Cut 1 item");
            }
        }

        // Paste
        KeyCode::Char('p') => {
            if app.clipboard.is_empty() {
                app.set_status("Clipboard is empty");
            } else {
                let dest = app.path.clone();
                let items = app.clipboard.clone();
                let total = crate::core::fs_ops::total_size(&items);
                let prog = std::sync::Arc::new(crate::core::fs_ops::OperationProgress::new(total));
                if app.clipboard_cut {
                    app.clipboard.clear();
                    crate::core::fs_ops::move_async(items, dest, prog);
                } else {
                    crate::core::fs_ops::copy_async(items, dest, prog);
                }
                app.set_status("Pasting…");
                app.needs_refresh = true;
            }
        }

        // Rename
        KeyCode::Char('r') | KeyCode::F(2) => {
            if let Some(f) = app.cursor_file().cloned() {
                app.input_buf = f.name.clone();
                app.mode = TuiMode::Rename;
            }
        }

        // Delete
        KeyCode::Char('d') | KeyCode::Delete => {
            if !app.selected.is_empty() || app.cursor_file().is_some() {
                app.mode = TuiMode::Delete;
            }
        }

        // Mkdir
        KeyCode::Char('m') => {
            app.input_buf = String::new();
            app.mode = TuiMode::Mkdir;
        }

        // New file
        KeyCode::Char('n') => {
            app.input_buf = String::new();
            app.mode = TuiMode::NewFile;
        }

        // Search
        KeyCode::Char('/') => {
            app.search_query.clear();
            app.cursor = 0;
            app.mode = TuiMode::Search;
        }

        // Toggle hidden
        KeyCode::Char('.') => {
            app.config.gui.show_hidden = !app.config.gui.show_hidden;
            app.needs_refresh = true;
            let h = if app.config.gui.show_hidden { "shown" } else { "hidden" };
            app.set_status(format!("Hidden files: {}", h));
        }

        // Cycle sort column
        KeyCode::Char('s') => {
            app.sort_col = match app.sort_col {
                SortColumn::Name => SortColumn::Size,
                SortColumn::Size => SortColumn::Modified,
                SortColumn::Modified => SortColumn::Kind,
                SortColumn::Kind => SortColumn::Permissions,
                SortColumn::Permissions => SortColumn::Name,
            };
            app.needs_refresh = true;
            app.set_status(format!("Sorted by: {:?}", app.sort_col));
        }

        // Toggle sort direction
        KeyCode::Char('S') => {
            app.sort_asc = !app.sort_asc;
            app.needs_refresh = true;
        }

        // Refresh
        KeyCode::F(5) => {
            app.needs_refresh = true;
        }

        // Help
        KeyCode::Char('?') => {
            app.mode = TuiMode::Help;
        }

        _ => {}
    }
    false
}

fn handle_search(app: &mut TuiApp, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Esc => {
            app.search_query.clear();
            app.mode = TuiMode::Browse;
            app.cursor = 0;
        }
        KeyCode::Enter => {
            app.mode = TuiMode::Browse;
        }
        KeyCode::Backspace => {
            app.search_query.pop();
            app.cursor = 0;
        }
        KeyCode::Up => {
            if app.cursor > 0 { app.cursor -= 1; }
        }
        KeyCode::Down => {
            let total = app.filtered_files().len();
            if app.cursor + 1 < total { app.cursor += 1; }
        }
        KeyCode::Char(c) => {
            app.search_query.push(c);
            app.cursor = 0;
        }
        _ => {}
    }
    false
}

fn handle_rename(app: &mut TuiApp, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Esc => { app.mode = TuiMode::Browse; }
        KeyCode::Enter => {
            if let Some(f) = app.cursor_file().cloned() {
                let new_path = f.path.parent().unwrap_or(&f.path).join(&app.input_buf);
                match rename_path(&f.path, &new_path) {
                    Ok(_) => app.set_status(format!("Renamed to '{}'", app.input_buf)),
                    Err(e) => app.set_status(format!("Error: {}", e)),
                }
                app.needs_refresh = true;
            }
            app.mode = TuiMode::Browse;
        }
        KeyCode::Backspace => { app.input_buf.pop(); }
        KeyCode::Char(c) => { app.input_buf.push(c); }
        _ => {}
    }
    false
}

fn handle_mkdir(app: &mut TuiApp, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Esc => { app.mode = TuiMode::Browse; }
        KeyCode::Enter => {
            let new_dir = app.path.join(&app.input_buf);
            match create_dir(&new_dir) {
                Ok(_) => app.set_status(format!("Created folder '{}'", app.input_buf)),
                Err(e) => app.set_status(format!("Error: {}", e)),
            }
            app.needs_refresh = true;
            app.mode = TuiMode::Browse;
        }
        KeyCode::Backspace => { app.input_buf.pop(); }
        KeyCode::Char(c) => { app.input_buf.push(c); }
        _ => {}
    }
    false
}

fn handle_new_file(app: &mut TuiApp, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Esc => { app.mode = TuiMode::Browse; }
        KeyCode::Enter => {
            let new_file = app.path.join(&app.input_buf);
            match create_file(&new_file) {
                Ok(_) => app.set_status(format!("Created '{}'", app.input_buf)),
                Err(e) => app.set_status(format!("Error: {}", e)),
            }
            app.needs_refresh = true;
            app.mode = TuiMode::Browse;
        }
        KeyCode::Backspace => { app.input_buf.pop(); }
        KeyCode::Char(c) => { app.input_buf.push(c); }
        _ => {}
    }
    false
}

fn handle_delete(app: &mut TuiApp, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Esc | KeyCode::Char('n') => {
            app.mode = TuiMode::Browse;
        }
        KeyCode::Enter | KeyCode::Char('y') => {
            let paths: Vec<_> = if !app.selected.is_empty() {
                app.selected.iter().cloned().collect()
            } else if let Some(f) = app.cursor_file().cloned() {
                vec![f.path]
            } else {
                vec![]
            };

            let count = paths.len();
            let mut errors = 0usize;
            for p in paths {
                if delete_path(&p).is_err() { errors += 1; }
            }
            app.selected.clear();
            if errors == 0 {
                app.set_status(format!("Deleted {} item(s)", count));
            } else {
                app.set_status(format!("Deleted with {} error(s)", errors));
            }
            app.needs_refresh = true;
            app.mode = TuiMode::Browse;
        }
        _ => {}
    }
    false
}

fn handle_editor(app: &mut TuiApp, key: KeyEvent) -> bool {
    let Some(ref mut editor) = app.editor else {
        app.mode = TuiMode::Browse;
        return false;
    };

    match key.code {
        // Save
        KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            match editor.save() {
                Ok(_) => app.set_status("Saved."),
                Err(e) => app.set_status(format!("Save error: {}", e)),
            }
        }
        // Close
        KeyCode::Esc => {
            app.editor = None;
            app.mode = TuiMode::Browse;
        }
        // Navigation
        KeyCode::Up => {
            if editor.cursor_row > 0 {
                editor.cursor_row -= 1;
                let line_len = editor.current_line().len();
                if editor.cursor_col > line_len { editor.cursor_col = line_len; }
                if editor.cursor_row < editor.scroll_offset {
                    editor.scroll_offset = editor.cursor_row;
                }
            }
        }
        KeyCode::Down => {
            if editor.cursor_row + 1 < editor.lines.len() {
                editor.cursor_row += 1;
                let line_len = editor.current_line().len();
                if editor.cursor_col > line_len { editor.cursor_col = line_len; }
            }
        }
        KeyCode::Left => {
            if editor.cursor_col > 0 { editor.cursor_col -= 1; }
        }
        KeyCode::Right => {
            let line_len = editor.current_line().len();
            if editor.cursor_col < line_len { editor.cursor_col += 1; }
        }
        KeyCode::Home => { editor.cursor_col = 0; }
        KeyCode::End => { editor.cursor_col = editor.current_line().len(); }
        KeyCode::Enter => { editor.newline(); }
        KeyCode::Backspace => { editor.backspace(); }
        KeyCode::Char(c) => { editor.insert_char(c); }
        _ => {}
    }
    false
}
