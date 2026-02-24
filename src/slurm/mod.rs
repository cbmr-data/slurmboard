mod config;
mod jobs;
mod misc;
mod nodes;
mod partitions;

use std::{collections::HashMap, rc::Rc};

pub use config::{DefaultMem, PartitionConfig, SlurmConfig};
pub use jobs::{Job, JobState};
pub use nodes::{CPUState, Node, NodeState};
pub use partitions::Partition;

use color_eyre::Result;

pub enum Identifier {
    Partition(String),
    Node(String),
}

pub struct Slurm {}

type JobMap = HashMap<String, Vec<Rc<Job>>>;
type NodeMap = HashMap<String, Vec<Rc<Node>>>;

impl Slurm {
    pub fn config() -> Result<SlurmConfig> {
        SlurmConfig::collect()
    }

    pub fn collect(config: &SlurmConfig) -> Result<Vec<Rc<Partition>>> {
        let (mut unallocated, mut allocated) = Slurm::collect_jobs()?;

        let mut partitions = NodeMap::new();
        for mut node in Node::collect()? {
            node.jobs = allocated.remove(&node.name).unwrap_or_default();

            node.default_mem = config.default_mem;
            if let Some(partition) = config.partitions.get(&node.partition.label) {
                if partition.default_mem != DefaultMem::Unlimited {
                    node.default_mem = partition.default_mem;
                }
            }

            partitions
                .entry(node.partition.label.clone())
                .or_default()
                .push(Rc::new(node));
        }

        let mut cluster: Vec<Rc<Partition>> = Vec::new();
        for (label, nodes) in partitions.drain() {
            assert!(!nodes.is_empty());

            let mut jobs = Vec::new();
            for node in &nodes {
                jobs.extend_from_slice(&node.jobs);
            }

            if let Some(unallocated) = unallocated.remove(&label) {
                jobs.extend_from_slice(&unallocated);
            }

            cluster.push(Rc::new(Partition {
                name: nodes[0].partition.clone(),
                jobs,
                nodes,
            }));
        }

        // Sort by descending number of nodes
        cluster.sort_by_key(|v| -(v.nodes.len() as isize));
        Ok(cluster)
    }

    fn collect_jobs() -> Result<(JobMap, JobMap)> {
        // FIXME: Warn on unassigned jobs
        let mut unallocated = JobMap::new();
        let mut allocated = JobMap::new();
        for job in Job::collect()? {
            let job = Rc::new(job);

            if job.nodelist.is_empty() {
                unallocated
                    .entry(job.partition.label.clone())
                    .or_default()
                    .push(job.clone());
            } else {
                for node in &job.nodelist {
                    allocated.entry(node.clone()).or_default().push(job.clone());
                }
            }
        }

        Ok((unallocated, allocated))
    }
}
