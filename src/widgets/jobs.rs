use std::{cmp::Reverse, fmt::Debug};

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Rect},
    style::{Color, Stylize},
    symbols::border,
    text::Text,
    widgets::{Block, Borders, StatefulWidgetRef, TableState, Widget},
};

use crate::slurm::{Job, JobState};
use crate::widgets::misc::scroll;

use super::{
    misc::{center_layout, mb_to_string, right_align_text},
    table::{GenericTable, GenericTableState},
};

#[derive(Clone, Copy, Debug)]
enum Column {
    JobID,
    User,
    State,
    Runtime,
    Nodes,
    Tasks,
    CPUs,
    GPUs,
    Memory,
    Nodelist,
    Name,
}

impl std::fmt::Display for Column {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self, f)
    }
}

#[derive(Debug)]
pub struct JobTableState {
    focus: bool,
    table: TableState,
    jobs: Vec<Job>,
    columns: Vec<Column>,
}

impl JobTableState {
    pub fn focus(&mut self, focus: bool) {
        self.focus = focus;
    }

    pub fn update(&mut self, jobs: &[Job]) {
        self.jobs.clear();
        self.jobs.extend_from_slice(jobs);
        self.jobs.sort_unstable_by_key(|j| Reverse(j.time.clone()));

        // Update/clear job selection depending on the new contents
        self.scroll(0);
    }

    pub fn scroll(&mut self, delta: isize) {
        scroll(&mut self.table, self.jobs.len(), delta);
    }

    pub fn click(&mut self, row: usize) {
        let offset = self.table.offset().saturating_add(row);
        self.table.select(Some(offset.saturating_sub(1)));
    }
}

impl Default for JobTableState {
    fn default() -> Self {
        Self {
            focus: false,
            columns: vec![
                Column::JobID,
                Column::User,
                Column::State,
                Column::Runtime,
                Column::Nodes,
                Column::Tasks,
                Column::CPUs,
                Column::GPUs,
                Column::Memory,
                Column::Nodelist,
                Column::Name,
            ],
            table: TableState::default(),
            jobs: Vec::default(),
        }
    }
}

impl GenericTableState<Column> for JobTableState {
    fn focus(&self) -> bool {
        self.focus
    }

    fn nrows(&self) -> usize {
        self.jobs.len()
    }

    fn columns(&self) -> &[Column] {
        &self.columns
    }

    fn selected(&self) -> Option<usize> {
        self.table.selected()
    }

    fn variable_width(&self, column: Column) -> bool {
        matches!(column, Column::Name)
    }

    fn text<'a>(&self, _constraint: &Constraint, row: usize, column: Column) -> Text<'a> {
        let job = &self.jobs[row];
        let text = match column {
            Column::JobID => job.id.to_string().into(),
            Column::User => job.user.clone().into(),
            Column::State => job.state.to_string().into(),
            Column::Runtime => right_align_text(&job.time),
            Column::Nodes => right_align_text(job.nodes),
            Column::Tasks => right_align_text(job.tasks),
            Column::CPUs => right_align_text(job.cpus),
            Column::GPUs => right_align_text(job.gpus),
            Column::Memory => mb_to_string(job.mem).into(),
            Column::Nodelist => Text::from(job.nodelist.join(",")),
            Column::Name => job.name.clone().into(),
        };

        if job.state != JobState::Running {
            text.fg(Color::Gray)
        } else {
            text
        }
    }

    fn inner_state(&mut self) -> &mut TableState {
        &mut self.table
    }
}

#[derive(Debug, Default)]
pub struct JobTable {}

impl JobTable {
    pub fn new() -> Self {
        Self::default()
    }

    // Renders a simple notification that there are no displayable jobs
    fn render_empty_table(area: Rect, buf: &mut Buffer) {
        let label = "No jobs found";
        // Size of label + surrounding border
        let width = label.chars().count() as u16 + 2;
        let height = 3;

        if let Some(area) = center_layout(area, width, height) {
            let block = Block::default()
                .borders(Borders::ALL)
                .border_set(border::PLAIN);

            Text::from(label).render(block.inner(area), buf);
            block.render(area, buf);
        }
    }
}

impl StatefulWidgetRef for JobTable {
    type State = JobTableState;

    fn render_ref(&self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        if state.jobs.is_empty() {
            Self::render_empty_table(area, buf)
        } else {
            let table = GenericTable::<Column, JobTableState>::new();

            table.render_ref(area, buf, state);
        }
    }
}
