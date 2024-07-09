use crate::slurm::Node;

use super::{jobs::Job, misc::unique_values, nodes::PartitionName};

#[derive(Clone, Debug)]
pub struct Partition {
    pub name: PartitionName,
    pub jobs: Vec<Job>,
    pub nodes: Vec<Node>,
}

impl Partition {
    pub fn users(&self) -> usize {
        unique_values(self.jobs.iter().map(|v| &v.user))
    }
}
