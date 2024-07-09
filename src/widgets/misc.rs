use std::fmt::Display;

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    text::Text,
    widgets::TableState,
};

pub const COLUMN_SPACING: u16 = 2;

pub fn scroll(state: &mut TableState, items: usize, delta: isize) -> Option<usize> {
    let selection = if items == 0 {
        None
    } else {
        Some(
            (state.selected().unwrap_or_default() as isize + delta).clamp(0, items as isize - 1)
                as usize,
        )
    };

    state.select(selection);
    selection
}

/// Right aligns displayable value
pub fn right_align_text<'a, T: Display>(v: T) -> Text<'a> {
    Text::from(v.to_string()).alignment(Alignment::Right)
}

/// Creates a `height`/`width` Rect centered in the specified `area`
pub fn center_layout(area: Rect, width: u16, height: u16) -> Option<Rect> {
    if width > area.width || height > area.height {
        return None;
    }

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(vec![
            Constraint::Length((area.height - height) / 2),
            Constraint::Length(height),
        ])
        .split(area);

    let layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(vec![
            Constraint::Length((area.width - width) / 2),
            Constraint::Length(width),
        ])
        .split(layout[1]);

    Some(layout[1])
}

pub fn mb_to_string(mb: usize) -> String {
    if mb < 1024 {
        format!("{}M", mb)
    } else if mb < 1048576 {
        format!("{:.1}G", mb as f64 / 1024.0)
    } else {
        format!("{:.1}T", mb as f64 / 1048576.0)
    }
}
