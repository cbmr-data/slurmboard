use color_eyre::Result;

use crate::{app::App, ui::UI};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};

/// Handles the key events and updates the state of [`App`].
pub fn handle_key_events(key_event: KeyEvent, app: &mut App, ui: &mut UI) -> Result<bool> {
    let mut processed = true;

    match key_event.code {
        // Exit application on `ESC` or `q`
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
            app.quit();
        }
        // Exit application on `Ctrl-C`
        KeyCode::Char('c') | KeyCode::Char('C') => {
            if key_event.modifiers == KeyModifiers::CONTROL {
                app.quit();
            } else {
                processed = false;
            }
        }
        // Toggle show/hide unavailable nodes
        KeyCode::Char('h') | KeyCode::Char('H') => {
            ui.toggle_unavailable();
        }
        // Force refresh of Slurm state
        KeyCode::Char('r') | KeyCode::Char('R') => {
            if app.update(1)? {
                ui.update(app);
            } else {
                processed = false;
            }
        }
        // Scrolling
        KeyCode::Home => ui.scroll(isize::MIN),
        KeyCode::PageUp => ui.scroll(-10),
        KeyCode::Up => ui.scroll(-1),
        KeyCode::Down => ui.scroll(1),
        KeyCode::PageDown => ui.scroll(10),
        KeyCode::End => ui.scroll(isize::MAX),
        // Sorting
        KeyCode::Left => ui.set_sort_column(-1),
        KeyCode::Right => ui.set_sort_column(1),
        KeyCode::Char('s') | KeyCode::Char('S') => {
            ui.toggle_sort_order();
        }
        // Switch focus between nodes / jobs
        KeyCode::Tab | KeyCode::BackTab => ui.toggle_focus(),
        _ => processed = false,
    }

    Ok(processed)
}

pub fn handle_mouse_events(event: MouseEvent, ui: &mut UI) -> Result<bool> {
    match event.kind {
        MouseEventKind::Down(MouseButton::Left) => ui.mouse_click(event.row),
        MouseEventKind::ScrollUp => ui.mouse_wheel(event.row, -1),
        MouseEventKind::ScrollDown => ui.mouse_wheel(event.row, 1),
        _ => return Ok(false),
    }

    Ok(true)
}
