use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Layout, Rect},
    symbols,
    widgets::{Block, StatefulWidgetRef, Widget},
};

use ratatui::{
    prelude::Stylize,
    symbols::border,
    text::Line,
    widgets::{
        block::{Position, Title},
        Borders,
    },
};

use crate::{
    app::App,
    widgets::{JobTable, JobTableState, NodeTable, NodeTableState, SelectionRef},
};

#[derive(Debug, Default, PartialEq, Eq)]
enum Focus {
    #[default]
    Jobs,
    Nodes,
}

#[derive(Debug, Default)]
pub struct UI {
    /// Indicates if the node list or job list has focus
    focus: Focus,
    nodes: NodeTable,
    node_state: NodeTableState,
    /// The last used layout; used to determine mouse-click targets
    node_layout: Rect,
    jobs: JobTable,
    job_state: JobTableState,
}

impl UI {
    pub fn new(app: &App) -> Self {
        let mut ui = Self::default();
        // Set the amount of memory allocated per CPU by default
        ui.node_state.set_def_mem_per_cpu(app.args.def_mem_per_cpu);
        // Set initial focus on node list
        ui.toggle_focus();
        // Fill out
        ui.update(app);
        ui
    }

    pub fn update(&mut self, app: &App) {
        self.node_state.update(app.cluster.clone());
        self.scroll_node_selection(0);
    }

    pub fn scroll(&mut self, delta: isize) {
        match self.focus {
            Focus::Nodes => self.scroll_node_selection(delta),
            Focus::Jobs => self.scroll_job_selection(delta),
        }
    }

    pub fn mouse_click(&mut self, row: u16) {
        if let Some(focus) = self.focus_at(row) {
            if self.focus != focus {
                self.toggle_focus();
            }

            match focus {
                Focus::Nodes => {
                    // -1 for border
                    self.node_state.click(row.saturating_sub(1) as usize);
                    self.scroll_node_selection(0)
                }
                Focus::Jobs => {
                    self.job_state
                        .click(row.saturating_sub(self.node_layout.height) as usize);
                    self.scroll_node_selection(0)
                }
            }
        }
    }

    pub fn mouse_wheel(&mut self, row: u16, delta: isize) {
        match self.focus_at(row) {
            Some(Focus::Jobs) => self.scroll_job_selection(delta),
            Some(Focus::Nodes) => self.scroll_node_selection(delta),
            None => {}
        }
    }

    pub fn toggle_focus(&mut self) {
        self.focus = match self.focus {
            Focus::Jobs => Focus::Nodes,
            Focus::Nodes => Focus::Jobs,
        };

        self.node_state.focus(self.focus == Focus::Nodes);
        self.job_state.focus(self.focus == Focus::Jobs);
    }

    pub fn toggle_unavailable(&mut self) {
        self.node_state.toggle_unavailable();
    }

    pub fn render(&mut self, area: Rect, buf: &mut Buffer) {
        // Require space for at least 4 rows, 2 headers, and 3 borders before rendering both tables
        if area.height >= 2 * (2 + 1) + 3 {
            let layout = Layout::default()
                .direction(ratatui::layout::Direction::Vertical)
                .constraints(vec![
                    // +2 for borders and an empty line to clearly indicate the end of the list
                    Constraint::Max((self.node_state.height() + 2).max(5)),
                    Constraint::Min(4),
                ])
                .split(area);

            self.render_nodes(layout[0], buf, Title::default());
            self.render_users(layout[1], buf, UI::instructions());
            self.node_layout = layout[0];
        } else {
            self.render_nodes(area, buf, UI::instructions());
            self.node_layout = area;
        }
    }

    fn focus_at(&self, row: u16) -> Option<Focus> {
        if row >= self.node_layout.height && !self.node_layout.is_empty() {
            Some(Focus::Jobs)
        } else if row < self.node_layout.height.saturating_sub(1) {
            Some(Focus::Nodes)
        } else {
            None
        }
    }

    /// Scrolls the node selection and updates the job-list
    fn scroll_node_selection(&mut self, delta: isize) {
        match self.node_state.scroll(delta) {
            Some(SelectionRef::Partition(partition)) => {
                self.job_state.update(&partition.jobs);
            }
            Some(SelectionRef::Node(node)) => {
                self.job_state.update(&node.jobs);
            }
            _ => self.job_state.update(&[]),
        }
    }

    /// Scrolls the job list
    fn scroll_job_selection(&mut self, delta: isize) {
        self.job_state.scroll(delta)
    }

    fn render_nodes(&mut self, area: Rect, buf: &mut Buffer, instructions: Title) {
        let title = vec![" Partitions ".bold()];
        let title = Title::from(Line::from(title));

        let block = Block::default()
            .title(title.clone().alignment(Alignment::Center))
            .title(instructions)
            .borders(Borders::TOP | Borders::LEFT | Borders::RIGHT)
            .border_set(border::PLAIN);

        self.nodes
            .render_ref(block.inner(area), buf, &mut self.node_state);
        block.render(area, buf);
    }

    fn render_users(&mut self, area: Rect, buf: &mut Buffer, instructions: Title) {
        let title = match self.node_state.selected() {
            Some(SelectionRef::Node(node)) => format!(" {} ", node.name),
            Some(SelectionRef::Partition(partition)) => format!(" {} ", partition.name),
            None => String::default(),
        };

        // Join border with border-less bottom of nodes table
        let border = symbols::border::Set {
            top_left: symbols::line::NORMAL.vertical_right,
            top_right: symbols::line::NORMAL.vertical_left,
            ..symbols::border::PLAIN
        };

        let block = Block::default()
            .title(Title::from(title).alignment(Alignment::Center))
            .title(instructions)
            .borders(Borders::ALL)
            .border_set(border);

        self.jobs
            .render_ref(block.inner(area), buf, &mut self.job_state);
        block.render(area, buf);
    }

    fn instructions() -> Title<'static> {
        Title::from(Line::from(vec![
            " <H> ".bold(),
            "Hide/Show unavailable".into(),
            " <R> ".bold(),
            "Refresh".into(),
            " <Q> ".bold(),
            "Quit ".into(),
        ]))
        .alignment(Alignment::Center)
        .position(Position::Bottom)
    }
}
