use std::{fmt::Debug, rc::Rc};

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Rect},
    style::{Color, Stylize},
    text::Text,
    widgets::{StatefulWidgetRef, TableState},
};

use crate::slurm::{Node, NodeState, Partition};
use crate::widgets::{misc::scroll, Utilization};

use super::{
    misc::right_align_text,
    table::{GenericTable, GenericTableState},
};

#[derive(Clone, Copy, Debug)]
pub enum NodeRow {
    Spacing,
    Partition(usize),
    Node(usize, usize),
}

#[derive(Clone, Copy, Debug)]
pub enum Selection<'a> {
    Partition(&'a Partition),
    Node(&'a Node),
}

#[derive(Clone, Copy, Debug)]
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
    cluster: Rc<Vec<Partition>>,
    /// Rows of nodes/partitions as indices into `cluster`, plus empty rows
    rows: Vec<NodeRow>,

    /// Value of DefMemPerCPU from /etc/slurm/slurm.conf
    def_mem_per_cpu: u64,
}

impl GenericTableState<Column> for NodeTableState {
    fn focus(&self) -> bool {
        self.focus
    }

    fn nrows(&self) -> usize {
        self.rows.len()
    }

    fn columns(&self) -> &[Column] {
        &self.columns
    }

    fn selected(&self) -> Option<usize> {
        self.table.selected()
    }

    fn variable_width(&self, column: Column) -> bool {
        matches!(column, Column::CPUs | Column::Memory)
    }

    fn text<'a>(&self, constraint: &Constraint, row: usize, column: Column) -> Text<'a> {
        match self.rows[row] {
            NodeRow::Partition(partition) => {
                self.partition_text(&self.cluster[partition], constraint, column)
            }
            NodeRow::Node(partition, node) => self.node_text(
                &self.cluster[partition].nodes[node],
                constraint,
                column,
                node == self.cluster[partition].nodes.len().saturating_sub(1),
            ),
            NodeRow::Spacing => Text::default(),
        }
    }

    fn inner_state(&mut self) -> &mut TableState {
        &mut self.table
    }
}

impl NodeTableState {
    pub fn set_def_mem_per_cpu(&mut self, def_mem_per_cpu: u64) {
        self.def_mem_per_cpu = def_mem_per_cpu;
    }

    pub fn focus(&mut self, focus: bool) {
        self.focus = focus;
    }

    pub fn scroll(&mut self, delta: isize) -> Option<Selection> {
        let items = &self.rows;
        loop {
            // Skip across across spacing elements
            if let Some(idx) = scroll(&mut self.table, items.len(), delta) {
                if !matches!(items[idx], NodeRow::Spacing)
                    || delta == 0
                    || (delta < 0 && idx == 0)
                    || (delta > 0 && idx + 1 >= items.len())
                {
                    break;
                }
            } else {
                break;
            }
        }

        self.selected()
    }

    pub fn selected(&self) -> Option<Selection> {
        if let Some(idx) = self.table.selected() {
            match self.rows[idx] {
                NodeRow::Partition(partition) => {
                    Some(Selection::Partition(&self.cluster[partition]))
                }
                NodeRow::Node(partition, node) => {
                    Some(Selection::Node(&self.cluster[partition].nodes[node]))
                }
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
        self.hide_unavailable = !self.hide_unavailable;
        self.update_selections();
    }

    pub fn update(&mut self, cluster: Rc<Vec<Partition>>) {
        self.cluster = cluster.clone();
        self.update_selections();
    }

    fn update_selections(&mut self) {
        self.rows.clear();

        for (p_idx, partition) in self.cluster.iter().enumerate() {
            self.rows.push(NodeRow::Partition(p_idx));

            for (n_idx, node) in partition.nodes.iter().enumerate() {
                if !self.hide_unavailable || node.state.is_available() {
                    self.rows.push(NodeRow::Node(p_idx, n_idx));
                }
            }

            self.rows.push(NodeRow::Spacing);
        }

        // Remove trailing spacing
        self.rows.pop();
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
                .map(|v| v.cpu_utilization(self.def_mem_per_cpu))
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
                    let mut gpus = v.gpu_utilization(self.def_mem_per_cpu);
                    if !v.state.is_available() {
                        gpus.allocated = 0.0;
                        gpus.utilized = 0.0;
                        gpus.blocked = 0.0;
                        gpus.unavailable = gpus.capacity;
                    }
                    gpus
                })
                .sum::<Utilization>()
                .to_line(constraint_length(*constraint))
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
                .cpu_utilization(self.def_mem_per_cpu)
                .to_line(constraint_length(*constraint))
                .into(),

            Column::Memory => node
                .mem_utilization()
                .to_line(constraint_length(*constraint))
                .into(),
            Column::GPUs => node
                .gpu_utilization(self.def_mem_per_cpu)
                .to_line(constraint_length(*constraint))
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
            cluster: Rc::default(),
            rows: Vec::default(),
            def_mem_per_cpu: 0,
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
    let color = if state.is_available() {
        Color::White
    } else {
        Color::Red
    };

    Text::from(state.to_string()).fg(color)
}

fn constraint_length(c: Constraint) -> u16 {
    match c {
        Constraint::Min(v) | Constraint::Max(v) | Constraint::Length(v) => v,
        _ => unimplemented!(),
    }
}
