//! Terminal User Interface (TUI) for Silicon Monitor
//!
//! This module provides an interactive terminal dashboard for real-time hardware monitoring.
//! It displays CPU, GPU, memory, disk, and system information using the ratatui library.

use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, prelude::Backend, Terminal};
use std::io;
use std::time::{Duration, Instant};

mod app;
mod ui;

pub use app::{AcceleratorInfo, AcceleratorType, App};

/// Run the TUI application
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and run
    let mut app = App::new()?;
    let res = run_app(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        eprintln!("Error: {:?}", err);
    }

    Ok(())
}

/// Main application loop
fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> Result<(), Box<dyn std::error::Error>> {
    let tick_rate = Duration::from_millis(500); // Update every 500ms
    let mut last_tick = Instant::now();

    // Create monitor for agent queries
    let monitor = crate::SiliconMonitor::new()?;

    loop {
        terminal.draw(|f| ui::draw(f, app))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    // Handle agent input mode separately
                    if app.agent_input_mode {
                        match key.code {
                            KeyCode::Char(c) => app.agent_input_char(c),
                            KeyCode::Backspace => app.agent_input_backspace(),
                            KeyCode::Enter => app.submit_agent_query(&monitor),
                            KeyCode::Esc => app.toggle_agent_input(),
                            _ => {}
                        }
                    } else {
                        match key.code {
                            KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                            KeyCode::Tab => {
                                // Tab cycles forward through process display modes
                                if key.modifiers.contains(KeyModifiers::SHIFT) {
                                    app.previous_process_mode();
                                } else {
                                    app.next_process_mode();
                                }
                            }
                            KeyCode::BackTab => {
                                // Shift+Tab (BackTab) cycles backward
                                app.previous_process_mode();
                            }
                            KeyCode::Char('1') => app.set_tab(0),
                            KeyCode::Char('2') => app.set_tab(1),
                            KeyCode::Char('3') => app.set_tab(2),
                            KeyCode::Char('4') => app.set_tab(3),
                            KeyCode::Char('5') => app.set_tab(4),
                            KeyCode::Char('6') => app.set_tab(5),
                            KeyCode::Left => app.previous_tab(),
                            KeyCode::Right => app.next_tab(),
                            KeyCode::Up => app.scroll_up(),
                            KeyCode::Down => app.scroll_down(),
                            KeyCode::Char('r') => app.reset_stats(),
                            KeyCode::Char('a') | KeyCode::Char('A') => app.toggle_agent_input(),
                            KeyCode::Char('c') | KeyCode::Char('C') => {
                                if app.selected_tab == 5 {
                                    app.clear_agent_history();
                                }
                            }
                            KeyCode::F(12) => {
                                if let Err(e) = app.save_config() {
                                    app.set_status_message(format!("Failed to save config: {}", e));
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            app.update()?;
            last_tick = Instant::now();
        }
    }
}
