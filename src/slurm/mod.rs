mod jobs;
mod misc;
mod nodes;
mod partitions;

use std::{collections::HashMap, rc::Rc};

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
    pub fn collect(sinfo: &str, squeue: &str) -> Result<Vec<Rc<Partition>>> {
        let mut jobs = Slurm::collect_jobs(squeue)?;

        let mut partitions: HashMap<String, Vec<Rc<Node>>> = HashMap::new();
        for mut node in Node::collect(sinfo)? {
            node.jobs = jobs.remove(&node.name).unwrap_or_default();

            partitions
                .entry(node.partition.label.clone())
                .or_default()
                .push(Rc::new(node));
        }

        let mut cluster: Vec<Rc<Partition>> = Vec::new();
        for (_, nodes) in partitions.drain() {
            assert!(!nodes.is_empty());

            let mut jobs = Vec::new();
            for node in &nodes {
                jobs.extend_from_slice(&node.jobs);
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

    fn collect_jobs(squeue: &str) -> Result<HashMap<String, Vec<Rc<Job>>>> {
        // FIXME: Warn on unassigned jobs
        let mut nodes: HashMap<String, Vec<Rc<Job>>> = HashMap::new();
        for job in Job::collect(squeue)? {
            let job = Rc::new(job);

            for node in &job.nodelist {
                nodes.entry(node.clone()).or_default().push(job.clone());
            }
        }

        Ok(nodes)
    }
}
