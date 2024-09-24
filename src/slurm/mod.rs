mod jobs;
mod misc;
mod nodes;
mod partitions;

pub use jobs::{Job, JobState};
pub use nodes::{CPUState, Node, NodeState};
pub use partitions::Partition;

use color_eyre::Result;

pub enum Identifier {
    Partition(String),
    Node(String),
}

pub struct Slurm {}

impl Slurm {
    pub fn collect(sinfo: &str, squeue: &str) -> Result<Vec<Partition>> {
        let partitions = Slurm::collect_partitions(sinfo)?;

        Slurm::collect_jobs(squeue, partitions)
    }

    fn collect_partitions(sinfo: &str) -> Result<Vec<Partition>> {
        let mut nodes = Node::collect(sinfo)?;
        nodes.sort_by_key(|v| (v.partition.to_string(), v.name.clone()));

        let mut partitions: Vec<Partition> = Vec::new();
        for node in nodes {
            if let Some(partition) = partitions.last_mut() {
                if partition.name.same(&node.partition) {
                    partition.nodes.push(node.clone());
                    continue;
                }
            }

            partitions.push(Partition {
                name: node.partition.clone(),
                nodes: vec![node.clone()],
                jobs: Vec::new(),
            });
        }

        // Sort by descending number of nodes
        partitions.sort_by_key(|v| -(v.nodes.len() as isize));
        Ok(partitions)
    }

    fn collect_jobs(squeue: &str, mut partitions: Vec<Partition>) -> Result<Vec<Partition>> {
        // FIXME: Warn on unassigned jobs
        for job in Job::collect(squeue)? {
            for partition in &mut partitions {
                if partition.name.same(&job.partition) {
                    partition.jobs.push(job.clone());

                    if !job.nodelist.is_empty() {
                        for node in &mut partition.nodes {
                            if job.nodelist.contains(&node.name) {
                                node.jobs.push(job.clone());
                            }
                        }
                    }

                    break;
                }
            }
        }

        Ok(partitions)
    }
}
