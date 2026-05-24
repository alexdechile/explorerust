use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Cell, Clear, Paragraph, Row, Table, TableState,
        Wrap,
    },
    Frame,
};

use super::app::{TuiApp, TuiMode};

const ACCENT: Color = Color::Cyan;
const DIM: Color = Color::DarkGray;
const HIGHLIGHT: Color = Color::Yellow;
const ERROR_COLOR: Color = Color::Red;
const DIR_COLOR: Color = Color::Blue;
const SYMLINK_COLOR: Color = Color::Green;

pub fn render(f: &mut Frame, app: &mut TuiApp) {
    match app.mode {
        TuiMode::Editor => render_editor(f, app),
        TuiMode::Help => render_help(f, app),
        _ => render_browser(f, app),
    }
}

fn render_browser(f: &mut Frame, app: &mut TuiApp) {
    let area = f.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // header/breadcrumb
            Constraint::Min(5),    // sidebar + main
            Constraint::Length(3), // status bar
        ])
        .split(area);

    render_header(f, app, chunks[0]);

    let body_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(22), Constraint::Min(10)])
        .split(chunks[1]);

    render_sidebar(f, app, body_chunks[0]);
    render_file_list(f, app, body_chunks[1]);
    render_status_bar(f, app, chunks[2]);

    // Overlay dialogs
    match app.mode {
        TuiMode::Search => render_search_bar(f, app, chunks[2]),
        TuiMode::Rename | TuiMode::Mkdir | TuiMode::NewFile => {
            render_input_dialog(f, app, area)
        }
        TuiMode::Delete => render_delete_confirm(f, app, area),
        _ => {}
    }
}

fn render_header(f: &mut Frame, app: &TuiApp, area: Rect) {
    // Build breadcrumb
    let mut spans: Vec<Span> = vec![
        Span::styled(" ExploreRust ", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
        Span::raw(" ❯ "),
    ];

    let components: Vec<_> = app.path.components().collect();
    let mut build = std::path::PathBuf::new();
    for (i, comp) in components.iter().enumerate() {
        build.push(comp);
        let name = if i == 0 { "/".to_string() } else { comp.as_os_str().to_string_lossy().to_string() };
        let style = if i == components.len() - 1 {
            Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(DIM)
        };
        spans.push(Span::styled(name, style));
        if i < components.len() - 1 {
            spans.push(Span::styled(" / ", Style::default().fg(DIM)));
        }
    }

    // Add hidden indicator
    if app.config.gui.show_hidden {
        spans.push(Span::styled("  [hidden]", Style::default().fg(HIGHLIGHT)));
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ACCENT))
        .title(" 📁 ExploreRust ");

    let p = Paragraph::new(Line::from(spans))
        .block(block)
        .alignment(Alignment::Left);

    f.render_widget(p, area);
}

fn render_sidebar(f: &mut Frame, app: &TuiApp, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(DIM))
        .title(" Quick Access ");

    let mut items: Vec<Line> = Vec::new();

    let quick = [
        ("🏠 Home", dirs::home_dir()),
        ("🖥 Desktop", dirs::desktop_dir()),
        ("📄 Docs", dirs::document_dir()),
        ("⬇ Downloads", dirs::download_dir()),
        ("🖼 Pictures", dirs::picture_dir()),
    ];

    for (label, dir) in &quick {
        if let Some(d) = dir {
            if d.exists() {
                let active = *d == app.path;
                let style = if active {
                    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };
                items.push(Line::from(Span::styled(format!(" {}", label), style)));
            }
        }
    }

    if !app.config.bookmarks.is_empty() {
        items.push(Line::from(Span::styled(" ─ Bookmarks ─", Style::default().fg(DIM))));
        for bm in &app.config.bookmarks {
            let name = bm.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| bm.to_string_lossy().to_string());
            let active = *bm == app.path;
            let style = if active {
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            items.push(Line::from(Span::styled(format!(" 🔖 {}", name), style)));
        }
    }

    let p = Paragraph::new(items).block(block);
    f.render_widget(p, area);
}

fn render_file_list(f: &mut Frame, app: &mut TuiApp, area: Rect) {
    let inner_height = area.height.saturating_sub(4) as usize; // borders + header row
    app.scroll_cursor(inner_height);

    let filtered = app.filtered_files();
    let display_files: Vec<&crate::core::fs_ops::FileEntry> = filtered
        .iter()
        .skip(app.scroll_offset)
        .take(inner_height.max(1))
        .copied()
        .collect();

    // Column headers
    let sort_arrow = |col: crate::core::fs_ops::SortColumn| -> &'static str {
        if app.sort_col == col {
            if app.sort_asc { " ▲" } else { " ▼" }
        } else { "" }
    };

    let header_cells = [
        format!("Name{}", sort_arrow(crate::core::fs_ops::SortColumn::Name)),
        format!("Size{}", sort_arrow(crate::core::fs_ops::SortColumn::Size)),
        format!("Type{}", sort_arrow(crate::core::fs_ops::SortColumn::Kind)),
        format!("Modified{}", sort_arrow(crate::core::fs_ops::SortColumn::Modified)),
        format!("Perms{}", sort_arrow(crate::core::fs_ops::SortColumn::Permissions)),
    ];
    let header = Row::new(header_cells.iter().map(|h| {
        Cell::from(h.as_str()).style(Style::default().fg(ACCENT).add_modifier(Modifier::BOLD))
    }));

    let rows: Vec<Row> = display_files.iter().enumerate().map(|(i, f)| {
        let abs_i = i + app.scroll_offset;
        let is_cursor = abs_i == app.cursor;
        let is_selected = app.selected.contains(&f.path);

        let name_color = if f.is_dir { DIR_COLOR }
            else if f.is_symlink { SYMLINK_COLOR }
            else { Color::White };

        let icon = file_icon(f);
        let name_span = format!("{} {}", icon, f.name);

        let row_style = if is_cursor {
            Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD)
        } else if is_selected {
            Style::default().bg(Color::Indexed(236))
        } else {
            Style::default()
        };

        let sel_marker = if is_selected { "●" } else { " " };

        Row::new(vec![
            Cell::from(format!("{} {}", sel_marker, name_span))
                .style(Style::default().fg(name_color)),
            Cell::from(f.size_display()).style(Style::default().fg(DIM)),
            Cell::from(f.kind_display()).style(Style::default().fg(DIM)),
            Cell::from(f.date_display()).style(Style::default().fg(DIM)),
            Cell::from(f.permissions.clone()).style(Style::default().fg(DIM)),
        ])
        .style(row_style)
    }).collect();

    let total = filtered.len();
    let title = format!(
        " {} items | {} selected | {:?} {} ",
        total,
        app.selected.len(),
        app.sort_col,
        if app.sort_asc { "▲" } else { "▼" }
    );

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ACCENT))
        .title(title);

    let mut table_state = TableState::default().with_selected(Some(app.cursor.saturating_sub(app.scroll_offset)));

    let table = Table::new(
        rows,
        [
            Constraint::Min(30),
            Constraint::Length(8),
            Constraint::Length(12),
            Constraint::Length(17),
            Constraint::Length(10),
        ],
    )
    .header(header)
    .block(block)
    .highlight_style(Style::default().add_modifier(Modifier::BOLD));

    f.render_stateful_widget(table, area, &mut table_state);
}

fn render_status_bar(f: &mut Frame, app: &TuiApp, area: Rect) {
    let help = " [↑↓] navigate  [Enter] open  [e] edit  [t] terminal  [/] search  [Space] select  [?] help  [q] quit";
    let status = if app.status.is_empty() { help } else { app.status.as_str() };

    let style = if app.status.contains("Error") {
        Style::default().fg(ERROR_COLOR)
    } else {
        Style::default().fg(DIM)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(DIM));

    let p = Paragraph::new(Span::styled(status, style)).block(block);
    f.render_widget(p, area);
}

fn render_search_bar(f: &mut Frame, app: &TuiApp, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(HIGHLIGHT))
        .title(" 🔍 Search ");

    let p = Paragraph::new(format!("/ {}", app.search_query)).block(block);
    f.render_widget(p, area);
}

fn render_input_dialog(f: &mut Frame, app: &TuiApp, area: Rect) {
    let title = match app.mode {
        TuiMode::Rename => " ✏ Rename ",
        TuiMode::Mkdir => " 📁 New Folder ",
        TuiMode::NewFile => " 📄 New File ",
        _ => " Input ",
    };

    let dialog = centered_rect(50, 20, area);
    f.render_widget(Clear, dialog);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(HIGHLIGHT))
        .title(title);

    let p = Paragraph::new(app.input_buf.as_str()).block(block);
    f.render_widget(p, dialog);
}

fn render_delete_confirm(f: &mut Frame, app: &TuiApp, area: Rect) {
    let dialog = centered_rect(60, 30, area);
    f.render_widget(Clear, dialog);

    let selected = app.selected.len().max(1);
    let text = format!(
        "\n  Delete {} item(s)?\n\n  This cannot be undone.\n\n  [Enter] Confirm   [Esc] Cancel",
        selected
    );

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ERROR_COLOR))
        .title(" ⚠ Confirm Delete ");

    let p = Paragraph::new(text)
        .block(block)
        .style(Style::default().fg(ERROR_COLOR));
    f.render_widget(p, dialog);
}

fn render_editor(f: &mut Frame, app: &TuiApp) {
    let Some(ref editor) = app.editor else { return };
    let area = f.area();

    let title = format!(
        " ✏ {} {} ",
        editor.path.file_name().unwrap_or_default().to_string_lossy(),
        if editor.modified { "●" } else { "" }
    );

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ACCENT))
        .title(title);

    let inner = block.inner(area);
    f.render_widget(block, area);

    let visible_height = inner.height as usize;
    let lines: Vec<Line> = editor.lines
        .iter()
        .enumerate()
        .skip(editor.scroll_offset)
        .take(visible_height)
        .map(|(i, line)| {
            let lineno = Span::styled(
                format!("{:4} ", i + 1),
                Style::default().fg(DIM),
            );
            Line::from(vec![lineno, Span::raw(line.as_str())])
        })
        .collect();

    let p = Paragraph::new(lines).wrap(Wrap { trim: false });
    f.render_widget(p, inner);

    // Status
    let status_area = Rect {
        x: inner.x,
        y: inner.y + inner.height.saturating_sub(1),
        width: inner.width,
        height: 1,
    };
    let status = Paragraph::new(Span::styled(
        " [Ctrl+S] Save  [Esc] Back ",
        Style::default().fg(DIM),
    ));
    f.render_widget(status, status_area);
}

fn render_help(f: &mut Frame, _app: &TuiApp) {
    let area = f.area();
    let dialog = centered_rect(70, 80, area);
    f.render_widget(Clear, dialog);

    let help_text = vec![
        Line::from(Span::styled(" ExploreRust — Keyboard Shortcuts", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from(Span::styled(" Navigation", Style::default().fg(HIGHLIGHT))),
        Line::from("  ↑ / k        Move cursor up"),
        Line::from("  ↓ / j        Move cursor down"),
        Line::from("  Enter / l    Open directory or file"),
        Line::from("  ← / h / BS   Go to parent directory"),
        Line::from("  Alt+←        Go back in history"),
        Line::from("  Alt+→        Go forward in history"),
        Line::from(""),
        Line::from(Span::styled(" File Operations", Style::default().fg(HIGHLIGHT))),
        Line::from("  Space        Toggle selection"),
        Line::from("  c            Copy selected"),
        Line::from("  x            Cut selected"),
        Line::from("  p            Paste"),
        Line::from("  r            Rename"),
        Line::from("  d / Delete   Delete (with confirm)"),
        Line::from("  m            New folder"),
        Line::from("  n            New file"),
        Line::from(""),
        Line::from(Span::styled(" Other", Style::default().fg(HIGHLIGHT))),
        Line::from("  e            Open in editor"),
        Line::from("  t            Open terminal here"),
        Line::from("  /            Search"),
        Line::from("  .            Toggle hidden files"),
        Line::from("  s            Cycle sort column"),
        Line::from("  S            Toggle sort direction"),
        Line::from("  F5           Refresh"),
        Line::from("  ?            This help"),
        Line::from("  q / Esc      Quit"),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ACCENT))
        .title(" ? Help ");

    let p = Paragraph::new(help_text).block(block);
    f.render_widget(p, dialog);
}

fn file_icon(f: &crate::core::fs_ops::FileEntry) -> &'static str {
    if f.is_dir { return "📁"; }
    if f.is_symlink { return "→"; }
    match f.extension.to_lowercase().as_str() {
        "rs" => "🦀", "md" | "markdown" => "📝", "txt" => "📄",
        "png" | "jpg" | "jpeg" | "gif" | "svg" => "🖼",
        "mp4" | "mkv" | "avi" => "🎬", "mp3" | "flac" | "ogg" => "🎵",
        "zip" | "tar" | "gz" | "xz" | "bz2" => "📦",
        "pdf" => "📕", "sh" | "bash" => "📜",
        _ => "📄",
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::vertical([
        Constraint::Percentage((100 - percent_y) / 2),
        Constraint::Percentage(percent_y),
        Constraint::Percentage((100 - percent_y) / 2),
    ])
    .split(r);

    Layout::horizontal([
        Constraint::Percentage((100 - percent_x) / 2),
        Constraint::Percentage(percent_x),
        Constraint::Percentage((100 - percent_x) / 2),
    ])
    .split(popup_layout[1])[1]
}
