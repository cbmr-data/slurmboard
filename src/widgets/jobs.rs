use std::{fmt::Debug, rc::Rc};

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Rect},
    style::{Color, Stylize},
    symbols::border,
    text::Text,
    widgets::{Block, Borders, StatefulWidgetRef, TableState, Widget},
};

use crate::widgets::misc::scroll;
use crate::{
    slurm::{Job, JobState},
    widgets::table::SortOrder,
};

use super::{
    misc::{center_layout, mb_to_string, right_align_text},
    table::{GenericTable, GenericTableState},
};

#[derive(Clone, Copy, Debug, PartialEq)]
enum Column {
    JobID,
    JobArray,
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
    jobs: Vec<Rc<Job>>,
    columns: Vec<Column>,
    sort_column: usize,
    sort_order: SortOrder,
}

impl JobTableState {
    pub fn focus(&mut self, focus: bool) {
        self.focus = focus;
    }

    pub fn update(&mut self, jobs: Vec<Rc<Job>>) {
        let selected = self.selected().map(|i| self.jobs[i].clone());

        self.jobs = jobs;
        self.sort();
        self.select(selected);
    }

    pub fn set_sort_column(&mut self, mut delta: isize) {
        delta = delta.saturating_add(self.sort_column as isize);
        if delta < 0 {
            self.sort_column = self.columns.len() - 1;
        } else if delta >= self.columns.len() as isize {
            self.sort_column = 0;
        } else {
            self.sort_column = delta as usize;
        }

        self.select(None);
        self.sort();
    }

    pub fn toggle_sort_order(&mut self) {
        self.sort_order = self.sort_order.toggle();

        self.select(None);
        self.sort();
    }

    fn sort(&mut self) {
        let cmp: fn(&Rc<Job>, &Rc<Job>) -> std::cmp::Ordering = match self.columns[self.sort_column]
        {
            Column::JobID => |a, b| a.id.cmp(&b.id),
            Column::JobArray => |a, b| a.array_job_id.cmp(&b.array_job_id),
            Column::User => |a, b| a.user.cmp(&b.user),
            Column::State => |a, b| a.state.cmp(&b.state),
            Column::Runtime => |a, b| a.time.cmp(&b.time),
            Column::Nodes => |a, b| a.nodes.cmp(&b.nodes),
            Column::Tasks => |a, b| a.tasks.cmp(&b.tasks),
            Column::CPUs => |a, b| a.cpus.cmp(&b.cpus),
            Column::GPUs => |a, b| a.gpus.cmp(&b.gpus),
            Column::Memory => |a, b| a.mem.cmp(&b.mem),
            Column::Nodelist => |a, b| a.nodelist.cmp(&b.nodelist),
            Column::Name => |a, b| a.name.cmp(&b.name),
        };

        match self.sort_order {
            SortOrder::Ascending => self.jobs.sort_by(cmp),
            SortOrder::Descending => self.jobs.sort_by(|a, b| cmp(a, b).reverse()),
        }
    }

    fn select(&mut self, job: Option<Rc<Job>>) {
        // Update/clear job selection depending on the new contents
        let selected = if self.jobs.is_empty() {
            None
        } else if let Some(job) = job {
            self.jobs
                .iter()
                .enumerate()
                .find(|v| v.1.id == job.id)
                .map(|v| v.0)
                .or(Some(0))
        } else {
            Some(0)
        };

        self.table.select(selected);
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
                Column::JobArray,
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
            sort_column: 4,
            sort_order: SortOrder::default(),
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

    fn sort_column(&self) -> Option<Column> {
        Some(self.columns[self.sort_column])
    }

    fn sort_order(&self) -> SortOrder {
        self.sort_order
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
            Column::JobArray => {
                if job.array_task_id != "N/A" {
                    if job.array_job_id != job.id {
                        format!("{} [{}]", job.array_job_id, job.array_task_id).into()
                    } else {
                        format!("[{}]", job.array_task_id).into()
                    }
                } else {
                    Text::default()
                }
            }
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
