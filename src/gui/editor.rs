use egui::Context;
use std::path::PathBuf;

use crate::config::EditorConfig;

// ── Editor state ──────────────────────────────────────────────────────────────

pub struct EditorState {
    pub path: PathBuf,
    pub content: String,
    pub modified: bool,
    pub is_markdown: bool,
    pub show_preview: bool,
    pub highlighter: Option<Highlighter>,
    pub config: EditorConfig,
    pub error: Option<String>,
    pub scroll_offset: f32,
}

impl EditorState {
    pub fn open(path: PathBuf, cfg: &EditorConfig) -> Self {
        let content = std::fs::read_to_string(&path).unwrap_or_default();
        let ext = path.extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        let is_markdown = matches!(ext.as_str(), "md" | "markdown");
        let highlighter = if cfg.syntax_highlight {
            Highlighter::new(&ext)
        } else {
            None
        };
        Self {
            path,
            content,
            modified: false,
            is_markdown,
            show_preview: is_markdown,
            highlighter,
            config: cfg.clone(),
            error: None,
            scroll_offset: 0.0,
        }
    }

    pub fn save(&mut self) {
        match std::fs::write(&self.path, &self.content) {
            Ok(_) => {
                self.modified = false;
                self.error = None;
            }
            Err(e) => {
                self.error = Some(e.to_string());
            }
        }
    }

    pub fn title(&self) -> String {
        let name = self.path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "untitled".into());
        if self.modified {
            format!("✏ {}  ●", name)
        } else {
            format!("✏ {}", name)
        }
    }
}

// ── Simple syntax highlighter ─────────────────────────────────────────────────

pub struct Highlighter {
    pub syntax_set: syntect::parsing::SyntaxSet,
    pub theme_set: syntect::highlighting::ThemeSet,
    pub extension: String,
}

impl Highlighter {
    pub fn new(ext: &str) -> Option<Self> {
        Some(Self {
            syntax_set: syntect::parsing::SyntaxSet::load_defaults_newlines(),
            theme_set: syntect::highlighting::ThemeSet::load_defaults(),
            extension: ext.to_string(),
        })
    }

    pub fn highlight(&self, code: &str, dark_mode: bool) -> egui::text::LayoutJob {
        use syntect::easy::HighlightLines;
        use syntect::util::LinesWithEndings;

        let theme_name = if dark_mode {
            "base16-ocean.dark"
        } else {
            "InspiredGitHub"
        };

        let syntax = self
            .syntax_set
            .find_syntax_by_extension(&self.extension)
            .or_else(|| Some(self.syntax_set.find_syntax_plain_text()))
            .unwrap();

        let theme = self
            .theme_set
            .themes
            .get(theme_name)
            .or_else(|| self.theme_set.themes.values().next())
            .unwrap();

        let mut h = HighlightLines::new(syntax, theme);
        let mut job = egui::text::LayoutJob::default();

        for line in LinesWithEndings::from(code) {
            if let Ok(ranges) = h.highlight_line(line, &self.syntax_set) {
                for (style, text) in ranges {
                    let color = egui::Color32::from_rgb(
                        style.foreground.r,
                        style.foreground.g,
                        style.foreground.b,
                    );
                    job.append(
                        text,
                        0.0,
                        egui::text::TextFormat {
                            color,
                            font_id: egui::FontId::monospace(13.0),
                            ..Default::default()
                        },
                    );
                }
            }
        }

        job
    }
}

// ── UI ────────────────────────────────────────────────────────────────────────

pub fn show(app: &mut super::app::ExploreApp, ctx: &Context) {
    // Clone minimal state to avoid borrow conflicts with the window closure
    let has_editor = app.editor.is_some();
    if !has_editor { return; }

    let title = app.editor.as_ref().map(|e| e.title()).unwrap_or_default();
    let is_markdown = app.editor.as_ref().map(|e| e.is_markdown).unwrap_or(false);
    let path_str = app.editor.as_ref()
        .map(|e| e.path.to_string_lossy().to_string())
        .unwrap_or_default();
    let err_str = app.editor.as_ref().and_then(|e| e.error.clone());
    let dark = app.dark_mode;
    let show_preview = app.editor.as_ref().map(|e| e.show_preview).unwrap_or(false);

    let mut open = true;
    let mut save_requested = false;
    let mut preview_toggle = show_preview;


    egui::Window::new(&title)
        .id(egui::Id::new("editor_window"))
        .default_size([800.0, 600.0])
        .min_size([400.0, 300.0])
        .resizable(true)
        .collapsible(false)
        .open(&mut open)
        .show(ctx, |ui| {
            // Toolbar
            ui.horizontal(|ui| {
                if ui.button("💾 Save").clicked() { save_requested = true; }
                if is_markdown {
                    ui.checkbox(&mut preview_toggle, "👁 Preview");
                }
                ui.separator();
                ui.label(&path_str);
                if let Some(ref err) = err_str {
                    ui.colored_label(egui::Color32::RED, format!("⚠ {}", err));
                }
            });

            let ctrl_s = ctx.input(|i| i.key_pressed(egui::Key::S) && i.modifiers.ctrl);
            if ctrl_s { save_requested = true; }

            ui.separator();

            if let Some(ref mut editor) = app.editor {
                editor.show_preview = preview_toggle;

                if is_markdown && editor.show_preview {
                    ui.columns(2, |cols| {
                        cols[0].heading("Edit");
                        egui::ScrollArea::vertical()
                            .id_source("editor_scroll")
                            .show(&mut cols[0], |ui| {
                                let response = ui.add(
                                    egui::TextEdit::multiline(&mut editor.content)
                                        .font(egui::TextStyle::Monospace)
                                        .desired_width(f32::INFINITY)
                                        .desired_rows(40)
                                        .code_editor(),
                                );
                                if response.changed() { editor.modified = true; }
                            });

                        cols[1].heading("Preview");
                        let content_copy = editor.content.clone();
                        egui::ScrollArea::vertical()
                            .id_source("preview_scroll")
                            .show(&mut cols[1], |ui| {
                                render_markdown(ui, &content_copy);
                            });
                    });
                } else {
                    egui::ScrollArea::vertical()
                        .id_source("editor_only_scroll")
                        .show(ui, |ui| {
                            let response = ui.add(
                                egui::TextEdit::multiline(&mut editor.content)
                                    .font(egui::FontId::monospace(editor.config.font_size))
                                    .desired_width(f32::INFINITY)
                                    .desired_rows(40)
                                    .code_editor(),
                            );
                            if response.changed() { editor.modified = true; }
                            let _ = dark;
                        });
                }
            }
        });

    if save_requested {
        if let Some(ref mut ed) = app.editor {
            ed.save();
        }
    }

    if !open {
        app.editor = None;
    }
}

// ── Markdown renderer (simple) ────────────────────────────────────────────────

fn render_markdown(ui: &mut egui::Ui, markdown: &str) {
    use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};

    let opts = Options::all();
    let parser = Parser::new_ext(markdown, opts);

    let mut in_code = false;
    let mut in_bold = false;
    let mut in_italic = false;
    let mut buf = String::new();
    let mut heading_level: Option<u8> = None;

    let flush_text = |ui: &mut egui::Ui, buf: &mut String, bold: bool, italic: bool, code: bool, heading: Option<u8>| {
        if buf.is_empty() { return; }
        let text = buf.clone();
        buf.clear();

        if code {
            ui.code(&text);
            return;
        }

        let rich = if let Some(level) = heading {
            let size = match level {
                1 => 24.0, 2 => 20.0, 3 => 17.0, _ => 15.0,
            };
            egui::RichText::new(&text).size(size).strong()
        } else if bold && italic {
            egui::RichText::new(&text).strong().italics()
        } else if bold {
            egui::RichText::new(&text).strong()
        } else if italic {
            egui::RichText::new(&text).italics()
        } else {
            egui::RichText::new(&text)
        };
        ui.label(rich);
    };

    for event in parser {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                heading_level = Some(match level {
                    HeadingLevel::H1 => 1, HeadingLevel::H2 => 2,
                    HeadingLevel::H3 => 3, HeadingLevel::H4 => 4,
                    HeadingLevel::H5 => 5, HeadingLevel::H6 => 6,
                });
            }
            Event::End(TagEnd::Heading(_)) => {
                flush_text(ui, &mut buf, in_bold, in_italic, in_code, heading_level);
                heading_level = None;
                ui.add_space(4.0);
            }
            Event::Start(Tag::Strong) => { in_bold = true; }
            Event::End(TagEnd::Strong) => {
                flush_text(ui, &mut buf, in_bold, in_italic, in_code, heading_level);
                in_bold = false;
            }
            Event::Start(Tag::Emphasis) => { in_italic = true; }
            Event::End(TagEnd::Emphasis) => {
                flush_text(ui, &mut buf, in_bold, in_italic, in_code, heading_level);
                in_italic = false;
            }
            Event::Start(Tag::CodeBlock(_)) => { in_code = true; }
            Event::End(TagEnd::CodeBlock) => {
                flush_text(ui, &mut buf, false, false, true, None);
                in_code = false;
                ui.add_space(4.0);
            }
            Event::Start(Tag::Paragraph) => {}
            Event::End(TagEnd::Paragraph) => {
                flush_text(ui, &mut buf, in_bold, in_italic, in_code, heading_level);
                ui.add_space(4.0);
            }
            Event::Start(Tag::Item) => {
                flush_text(ui, &mut buf, in_bold, in_italic, in_code, heading_level);
                buf.push_str("• ");
            }
            Event::End(TagEnd::Item) => {
                flush_text(ui, &mut buf, in_bold, in_italic, in_code, heading_level);
            }
            Event::Rule => {
                ui.separator();
            }
            Event::SoftBreak | Event::HardBreak => {
                flush_text(ui, &mut buf, in_bold, in_italic, in_code, heading_level);
            }
            Event::Text(text) => {
                buf.push_str(&text);
            }
            Event::Code(text) => {
                ui.code(text.as_ref());
            }
            _ => {}
        }
    }

    if !buf.is_empty() {
        flush_text(ui, &mut buf, in_bold, in_italic, in_code, heading_level);
    }
}
