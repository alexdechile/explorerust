mod app;
mod events;
mod ui;

use anyhow::Result;
use std::path::PathBuf;

pub fn run(start_path: PathBuf) -> Result<()> {
    use crossterm::{
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    };
    use ratatui::backend::CrosstermBackend;
    use ratatui::Terminal;
    use std::io;

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = app::TuiApp::new(start_path);
    let result = run_loop(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn run_loop<B: ratatui::backend::Backend>(
    terminal: &mut ratatui::Terminal<B>,
    app: &mut app::TuiApp,
) -> Result<()> {
    use crossterm::event::{self, Event, KeyEventKind};
    use std::time::Duration;

    loop {
        terminal.draw(|f| ui::render(f, app))?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    if events::handle(app, key) {
                        break; // quit
                    }
                }
            }
        }

        // Refresh if needed
        if app.needs_refresh {
            app.refresh();
            app.needs_refresh = false;
        }
    }

    Ok(())
}
