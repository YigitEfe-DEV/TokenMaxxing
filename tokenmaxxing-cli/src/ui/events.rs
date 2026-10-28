//! The event loop of the dashboard.
//!
//! The loop waits for a key press, but it looks at the background work in
//! between, so a scan of a large repository does not freeze the display.

use std::io;
use std::time::Duration;

use crossterm::event::{self, Event, KeyEventKind};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use super::app::App;
use super::render;

/// How long the loop waits for a key before it looks at the background work.
const POLL_INTERVAL: Duration = Duration::from_millis(120);

/// Runs the dashboard until the user leaves it.
///
/// The terminal is restored even when the loop fails, so a crash never leaves
/// the terminal in the alternate screen.
pub fn run(app: &mut App) -> io::Result<()> {
    let mut terminal = ratatui::try_init()?;
    let result = event_loop(&mut terminal, app);
    let restored = ratatui::try_restore();

    result.and(restored)
}

/// The loop itself.
fn event_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> io::Result<()> {
    while !app.should_quit {
        terminal.draw(|frame| render::draw(frame, app))?;

        if event::poll(POLL_INTERVAL)? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => app.handle_key(key),
                Event::Resize(_, _) => {}
                _ => {}
            }
        }

        app.poll_background();
    }

    Ok(())
}
