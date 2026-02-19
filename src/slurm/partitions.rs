use std::rc::Rc;

use crate::slurm::Node;

use super::{jobs::Job, misc::unique_values, nodes::PartitionName};

#[derive(Clone, Debug)]
pub struct Partition {
    pub name: PartitionName,
    pub jobs: Vec<Rc<Job>>,
    pub nodes: Vec<Rc<Node>>,
}

impl Partition {
    pub fn users(&self) -> usize {
        unique_values(self.jobs.iter().map(|v| &v.user))
    }
}
