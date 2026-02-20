use std::{fmt::Debug, rc::Rc};

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Rect},
    style::{Color, Stylize},
    text::Text,
    widgets::{StatefulWidgetRef, TableState},
};

use crate::widgets::{misc::scroll, Utilization};
use crate::{
    slurm::{Node, NodeState, Partition},
    widgets::table::SortOrder,
};

use super::{
    misc::right_align_text,
    table::{GenericTable, GenericTableState},
};

#[derive(Clone, Debug)]
pub enum NodeRow {
    Spacing,
    Partition(Rc<Partition>),
    Node(Rc<Node>),
}

#[derive(Clone, Debug)]
pub enum Selection {
    Partition(Rc<Partition>),
    Node(Rc<Node>),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Column {
    Node,
    State,
    Users,
    Jobs,
    CPUs,
    Memory,
    GPUs,
}

impl std::fmt::Display for Column {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self, f)
    }
}

#[derive(Debug)]
pub struct NodeTableState {
    /// Does this widget have focus?
    focus: bool,
    /// Should unavailable nodes be hidden?
    hide_unavailable: bool,
    /// Visible columns
    columns: Vec<Column>,
    table: TableState,
    cluster: Vec<Rc<Partition>>,
    /// Rows of nodes/partitions as indices into `cluster`, plus empty rows
    rows: Vec<NodeRow>,

    /// Total number of GPUs
    gpus: usize,
}

impl GenericTableState<Column> for NodeTableState {
    fn focus(&self) -> bool {
        self.focus
    }

    fn nrows(&self) -> usize {
        self.rows.len()
    }

    fn columns(&self) -> &[Column] {
        if self.gpus > 0 {
            &self.columns
        } else {
            &self.columns[..self.columns.len() - 1]
        }
    }

    fn sort_column(&self) -> Option<Column> {
        None
    }

    fn sort_order(&self) -> SortOrder {
        SortOrder::default()
    }

    fn selected(&self) -> Option<usize> {
        self.table.selected()
    }

    fn variable_width(&self, column: Column) -> bool {
        matches!(column, Column::CPUs | Column::Memory)
    }

    fn text<'a>(&self, constraint: &Constraint, row: usize, column: Column) -> Text<'a> {
        match &self.rows[row] {
            NodeRow::Partition(partition) => self.partition_text(partition, constraint, column),
            NodeRow::Node(node) => {
                let last =
                    row + 1 == self.rows.len() || matches!(self.rows[row + 1], NodeRow::Spacing);

                self.node_text(node, constraint, column, last)
            }
            NodeRow::Spacing => Text::default(),
        }
    }

    fn inner_state(&mut self) -> &mut TableState {
        &mut self.table
    }
}

impl NodeTableState {
    pub fn focus(&mut self, focus: bool) {
        self.focus = focus;
    }

    pub fn scroll(&mut self, mut delta: isize) -> Option<Selection> {
        // Skip across across spacing elements
        while let Some(idx) = scroll(&mut self.table, self.rows.len(), delta) {
            if !matches!(self.rows[idx], NodeRow::Spacing)
                || delta == 0
                || (delta < 0 && idx == 0)
                || (delta > 0 && idx + 1 >= self.rows.len())
            {
                break;
            }

            delta = delta.clamp(-1, 1);
        }

        self.selected()
    }

    pub fn selected(&self) -> Option<Selection> {
        if let Some(idx) = self.table.selected() {
            match &self.rows[idx] {
                NodeRow::Partition(partition) => Some(Selection::Partition(partition.clone())),
                NodeRow::Node(node) => Some(Selection::Node(node.clone())),
                NodeRow::Spacing => None,
            }
        } else {
            None
        }
    }

    pub fn click(&mut self, row: usize) {
        let offset = self.table.offset().saturating_add(row).saturating_sub(1);
        if let Some(selection) = self.rows.get(offset) {
            if !matches!(selection, NodeRow::Spacing) {
                self.table.select(Some(offset));
            }
        }
    }

    pub fn toggle_unavailable(&mut self) {
        let selection = self.selected();
        self.hide_unavailable = !self.hide_unavailable;
        self.update_table();
        self.select(selection)
    }

    pub fn update(&mut self, cluster: Vec<Rc<Partition>>) {
        let selection = self.selected();
        self.cluster = cluster;
        self.update_table();
        self.select(selection);

        self.gpus = 0;
        for partition in &self.cluster {
            for node in &partition.nodes {
                self.gpus += node.gpus;
            }
        }
    }

    fn update_table(&mut self) {
        self.rows.clear();

        for partition in &self.cluster {
            self.rows.push(NodeRow::Partition(partition.clone()));

            for node in &partition.nodes {
                if !self.hide_unavailable || node.state.is_available() {
                    self.rows.push(NodeRow::Node(node.clone()));
                }
            }

            self.rows.push(NodeRow::Spacing);
        }

        // Remove trailing spacing
        self.rows.pop();
    }

    fn select(&mut self, selection: Option<Selection>) {
        if let Some(selection) = selection {
            for (idx, candidate) in self.rows.iter().enumerate() {
                match (&selection, candidate) {
                    (Selection::Node(selection), NodeRow::Node(candidate)) => {
                        if selection.name == candidate.name {
                            self.table.select(Some(idx));
                            return;
                        }
                    }
                    (Selection::Partition(selection), NodeRow::Partition(candidate)) => {
                        if selection.name.same(&candidate.name) {
                            self.table.select(Some(idx));
                            return;
                        }
                    }
                    _ => {}
                }
            }

            // Fall back to selecting the same partition
            let partition = match &selection {
                Selection::Node(selection) => &selection.partition,
                Selection::Partition(selection) => &selection.name,
            };

            for (idx, candidate) in self.rows.iter().enumerate() {
                if let NodeRow::Partition(candidate) = candidate {
                    if candidate.name.same(partition) {
                        self.table.select(Some(idx));
                        return;
                    }
                }
            }
        }

        self.table.select(None)
    }

    pub fn height(&self) -> u16 {
        self.rows.len() as u16 + 1 // +1 for headers
    }

    fn partition_text<'a>(
        &self,
        partition: &Partition,
        constraint: &Constraint,
        column: Column,
    ) -> Text<'a> {
        match column {
            Column::Node => partition.name.to_string().into(),
            Column::State => Text::default(),
            Column::Users => right_align_text(partition.users()),
            Column::Jobs => right_align_text(partition.jobs.len()),
            Column::CPUs => partition
                .nodes
                .iter()
                .map(|v| v.cpu_utilization())
                .sum::<Utilization>()
                .to_line(constraint_length(*constraint))
                .into(),
            Column::Memory => {
                partition
                    .nodes
                    .iter()
                    .map(|v| {
                        let mut mem = v.mem_utilization();
                        if !v.state.is_available() {
                            // Slurm doesn't track availability of RAM, but we consider
                            // RAM unavailable if the node is unavailable.
                            mem.allocated = 0.0;
                            mem.utilized = 0.0;
                            mem.blocked = 0.0;
                            mem.unavailable = mem.capacity;
                        }
                        mem
                    })
                    .sum::<Utilization>()
                    .to_line(constraint_length(*constraint))
                    .into()
            }
            Column::GPUs => partition
                .nodes
                .iter()
                .map(|v| {
                    let mut gpus = v.gpu_utilization();
                    if !v.state.is_available() {
                        gpus.allocated = 0.0;
                        gpus.utilized = 0.0;
                        gpus.blocked = 0.0;
                        gpus.unavailable = gpus.capacity;
                    }
                    gpus
                })
                .sum::<Utilization>()
                .to_line(gpu_column_width(self.gpus))
                .into(),
        }
    }

    fn node_text<'a>(
        &self,
        node: &Node,
        constraint: &Constraint,
        column: Column,
        last: bool,
    ) -> Text<'a> {
        match column {
            Column::Node => Text::from(format!(" {} {}", if last { "┕" } else { "┝" }, node.name)),
            Column::State => color_state_text(&node.state),
            Column::Users => right_align_text(node.users()),
            Column::Jobs => right_align_text(node.jobs.len()),
            Column::CPUs => node
                .cpu_utilization()
                .to_line(constraint_length(*constraint))
                .into(),

            Column::Memory => node
                .mem_utilization()
                .to_line(constraint_length(*constraint))
                .into(),
            Column::GPUs => node
                .gpu_utilization()
                .to_line(gpu_column_width(self.gpus))
                .into(),
        }
    }
}

impl Default for NodeTableState {
    fn default() -> Self {
        Self {
            focus: false,
            hide_unavailable: false,
            columns: vec![
                Column::Node,
                Column::State,
                Column::Users,
                Column::Jobs,
                Column::CPUs,
                Column::Memory,
                Column::GPUs,
            ],
            table: TableState::default(),
            cluster: Vec::default(),
            rows: Vec::default(),
            gpus: 0,
        }
    }
}

#[derive(Debug, Default)]
pub struct NodeTable {}

impl NodeTable {
    pub fn new() -> NodeTable {
        NodeTable::default()
    }
}

impl StatefulWidgetRef for NodeTable {
    type State = NodeTableState;

    fn render_ref(&self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let table = GenericTable::<Column, NodeTableState>::new();

        table.render_ref(area, buf, state);
    }
}

/// Colorize a Node state based on availability
fn color_state_text<'a>(state: &NodeState) -> Text<'a> {
    let text = Text::from(state.to_string());

    if state.is_available() {
        text.dim()
    } else {
        text.fg(Color::Red)
    }
}

fn constraint_length(c: Constraint) -> u16 {
    match c {
        Constraint::Min(v) | Constraint::Max(v) | Constraint::Length(v) => v,
        _ => unimplemented!(),
    }
}

// Returns a width
fn gpu_column_width(gpus: usize) -> u16 {
    gpus.clamp(16, 32) as u16
}
