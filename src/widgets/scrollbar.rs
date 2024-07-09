use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    widgets::{Scrollbar, ScrollbarOrientation, ScrollbarState, StatefulWidget},
};

#[derive(Debug, Default)]
pub struct RightScrollbar {
    items: usize,
    header: usize,
    selected: usize,
}

impl RightScrollbar {
    pub fn items(mut self, items: usize) -> Self {
        self.items = items;
        self
    }

    pub fn header(mut self, header: usize) -> Self {
        self.header = header;
        self
    }

    pub fn selected(mut self, idx: Option<usize>) -> Self {
        self.selected = idx.unwrap_or_default();
        self
    }

    pub fn render(self, area: Rect, buf: &mut Buffer) -> Rect {
        let mut state = ScrollbarState::default()
            .content_length(self.items)
            .position(self.selected);

        let main_layout = Layout::default()
            .direction(ratatui::layout::Direction::Horizontal)
            .constraints(vec![Constraint::Percentage(100), Constraint::Length(2)])
            .split(area);

        let layout = Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints(vec![
                Constraint::Length(self.header as u16),
                Constraint::Percentage(100),
            ])
            .split(main_layout[1]);

        Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .end_symbol(None)
            .track_symbol(None)
            .thumb_symbol("▐")
            .render(layout[1], buf, &mut state);

        main_layout[0]
    }
}
