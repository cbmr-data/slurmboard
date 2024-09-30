use std::fmt;
use std::process::Command;
use std::str::{FromStr, Split};

use color_eyre::eyre::Context;
use color_eyre::Result;
use serde::{Deserialize, Deserializer};

use serde::de::{self, IntoDeserializer, Visitor};

use crate::widgets::Utilization;

use super::jobs::Job;
use super::misc::{format_string, unique_values};

/// Summarizes the state of CPUs on a node
#[derive(Clone, Debug, Default)]
pub struct CPUState {
    /// Allocated CPUs
    pub allocated: usize,
    /// Idle CPUs
    pub idle: usize,
    /// Unavailable CPUs
    pub other: usize,
    /// Total number of CPUs
    pub total: usize,
}

impl<'de> Deserialize<'de> for CPUState {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(CPUStateVisitor)
    }
}

struct CPUStateVisitor;

impl<'de> Visitor<'de> for CPUStateVisitor {
    type Value = CPUState;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a string representing CPU states in the form '0/1/2/3'")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        fn parse_next<E>(s: &mut Split<char>) -> Result<usize, E>
        where
            E: de::Error,
        {
            let value = s
                .next()
                .ok_or_else(|| E::custom("number of CPUs not found"))?;

            value
                .parse::<usize>()
                .map_err(|_| E::custom(format!("{:?} is not a valid number of CPUs", value)))
        }

        let mut values: Split<char> = v.split('/');
        Ok(CPUState {
            allocated: parse_next(&mut values)?,
            idle: parse_next(&mut values)?,
            other: parse_next(&mut values)?,
            total: parse_next(&mut values)?,
        })
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SlurmState {
    Allocated,
    Completing,
    Down,
    Drained,
    Draining,
    Fail,
    Failing,
    Future,
    Idle,
    #[serde(rename = "maint")]
    Maintenance,
    Mixed,
    Perfctrs,
    PowerDown,
    PowerUp,
    Reserved,
    Unknown,
}

#[derive(Clone, Debug)]
pub struct PartitionName {
    /// Made of a partition
    pub label: String,
    /// Indicates the default partition; this flag is explicitly ignored
    /// as it is purely used for formatting purposes to match sinfo/squeue
    pub default: bool,
}

impl PartitionName {
    /// Returns the length of the name including (optional) default marker
    pub fn len(&self) -> usize {
        self.label.len() + if self.default { 1 } else { 0 }
    }

    /// Trims the trailing '*' indicating that a partition is the default partition
    pub fn from_str<'de, D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value: &str = Deserialize::deserialize(deserializer)?;

        Ok(Self {
            label: value.trim_end_matches('*').to_string(),
            default: value.ends_with('*'),
        })
    }

    /// Indicates if two nodes are the same, ignoring the 'default' flag
    pub fn same(&self, other: &Self) -> bool {
        self.label == other.label
    }
}

impl fmt::Display for PartitionName {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.default {
            write!(f, "{}*", self.label)
        } else {
            fmt::Display::fmt(&self.label, f)
        }
    }
}

#[derive(Clone, Debug)]
pub struct NodeState {
    pub state: SlurmState,
    pub responds: bool,
}

impl NodeState {
    fn from_str<'de, D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // Trim optional
        let value: &str = Deserialize::deserialize(deserializer)?;
        let state = value.trim_matches('*');

        Ok(NodeState {
            state: SlurmState::deserialize(state.into_deserializer())?,
            responds: !value.ends_with('*'),
        })
    }

    /// Returns true if the node is available for executing jobs
    pub fn is_available(&self) -> bool {
        self.responds
            && matches!(
                self.state,
                SlurmState::Allocated
                    | SlurmState::Completing
                    | SlurmState::Idle
                    | SlurmState::Mixed
                    | SlurmState::Reserved
            )
    }
}

impl fmt::Display for NodeState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.responds {
            fmt::Debug::fmt(&self.state, f)
        } else {
            write!(f, "{:?}*", self.state)
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct Node {
    #[serde(rename = "NODELIST")]
    pub name: String,
    #[serde(rename = "PARTITION", deserialize_with = "PartitionName::from_str")]
    pub partition: PartitionName,
    #[serde(rename = "STATE", deserialize_with = "NodeState::from_str")]
    pub state: NodeState,

    #[serde(rename = "CPUS")]
    pub cpus: usize,
    #[serde(rename = "CPU_LOAD", deserialize_with = "parse_cpu_load")]
    pub cpu_load: Option<f64>,
    #[serde(rename = "CPUS(A/I/O/T)")]
    pub cpu_state: CPUState,

    #[serde(rename = "MEMORY")]
    pub mem: usize,
    #[serde(rename = "ALLOCMEM")]
    pub mem_alloc: usize,
    #[serde(rename = "FREE_MEM", deserialize_with = "parse_free_mem")]
    pub mem_free: Option<usize>,

    #[serde(skip_deserializing)]
    pub gpus: usize,
    #[serde(skip_deserializing)]
    pub gpus_used: usize,

    #[serde(rename = "GRES")]
    gres: String,
    #[serde(rename = "GRES_USED")]
    gres_used: String,

    #[serde(skip)]
    pub jobs: Vec<Job>,
}

impl Node {
    pub fn users(&self) -> usize {
        unique_values(self.jobs.iter().map(|v| &v.user))
    }

    pub fn cpu_utilization(&self, mem_per_cpu: u64) -> Utilization {
        // CPU load is refreshed at a slow pace, resulting in load frequently
        // exceeding the number of CPUs allocated; for this reason the value
        // is capped at the number of CPUs reserved.
        let utilized = self
            .cpu_load
            .unwrap_or(0.0)
            .min(self.cpu_state.allocated as f64);

        // Reserved RAM "blocks" the allocation of CPUs, unless the end-user
        // explicitly requests less RAM per CPU for a job.
        let blocked = if mem_per_cpu > 0 {
            (self.mem_alloc as f64 / mem_per_cpu as f64).ceil()
        } else {
            0.0
        };

        Utilization {
            utilized,
            allocated: self.cpu_state.allocated as f64,
            blocked: blocked.max(self.cpu_state.allocated as f64),
            unavailable: self.cpu_state.other as f64,
            capacity: self.cpu_state.total as f64,
        }
    }

    pub fn mem_utilization(&self) -> Utilization {
        // See note regarding CPU load above
        // Free memory includes memory not allocated for Slurm
        let utilized = self
            .mem
            .saturating_sub(self.mem_free.unwrap_or(self.mem))
            .min(self.mem_alloc) as f64;
        // Memory is considered "blocked" if there are no CPUs available for allocation
        let (blocked, unavailable) =
            if self.cpu_state.allocated + self.cpu_state.other < self.cpu_state.total {
                (self.mem_alloc as f64, 0.0)
            } else if self.cpu_state.total.saturating_sub(self.cpu_state.other) > 0 {
                (self.mem as f64, 0.0)
            } else {
                (0.0, self.mem.saturating_sub(self.mem_alloc) as f64)
            };

        Utilization {
            utilized,
            allocated: self.mem_alloc as f64,
            blocked,
            unavailable,
            capacity: self.mem as f64,
        }
    }

    pub fn gpu_utilization(&self, mem_per_cpu: u64) -> Utilization {
        let cpu_utilization = self.cpu_utilization(mem_per_cpu);

        // GPUs are considered blocked if there are no available CPUs assuming default RAM allocations
        let blocked = if cpu_utilization.available() < 1.0 {
            self.gpus - self.gpus_used
        } else {
            0
        };

        Utilization {
            utilized: 0.0,
            allocated: self.gpus_used as f64,
            blocked: blocked as f64,
            unavailable: 0.0,
            capacity: self.gpus as f64,
        }
    }

    pub fn collect(exe: &str) -> Result<Vec<Node>> {
        let output = Command::new(exe)
            .args(["-N", "--Format", &sinfo_format()])
            .output()
            .wrap_err("failed to execute squeue")?;

        // TODO: check output.status
        Self::parse(std::io::Cursor::new(output.stdout))
    }

    /// Parses a CSV file into a vector of `Node`
    fn parse<R>(reader: R) -> Result<Vec<Node>>
    where
        R: std::io::Read,
    {
        let mut nodes = Vec::new();
        for node in csv::ReaderBuilder::new()
            .delimiter(b'|')
            .from_reader(reader)
            .deserialize::<Node>()
        {
            let mut node = node.wrap_err("error while parsing sinfo output")?;
            node.gpus = parse_gpus(&node.gres).wrap_err("parsing GRES")?;
            node.gpus_used = parse_gpus(&node.gres_used).wrap_err("parsing GRES_USED")?;

            nodes.push(node);
        }

        Ok(nodes)
    }
}

/// Generates parameter for the `-F` command-line option for `sinfo`
fn sinfo_format() -> String {
    format_string(
        [
            "AllocMem",
            "CPUs",
            "CPUsLoad",
            "CPUsState",
            "FreeMem",
            "Gres",
            "GresUsed",
            "Memory",
            "NodeList",
            "Partition",
            "StateLong",
        ]
        .iter(),
    )
}

fn parse_optional_value<'de, D, T>(name: &str, deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: FromStr,
{
    let value: &str = Deserialize::deserialize(deserializer)?;
    if value == "N/A" {
        return Ok(None);
    }

    Ok(Some(value.parse::<T>().map_err(|_| {
        de::Error::custom(format!("invalid {}: {:?}", name, value))
    })?))
}

fn parse_cpu_load<'de, D>(deserializer: D) -> Result<Option<f64>, D::Error>
where
    D: Deserializer<'de>,
{
    parse_optional_value("CPU_LOAD", deserializer)
}

fn parse_free_mem<'de, D>(deserializer: D) -> Result<Option<usize>, D::Error>
where
    D: Deserializer<'de>,
{
    parse_optional_value("FREE_MEM", deserializer)
}

fn parse_gpus(tres: &str) -> Result<usize> {
    for value in tres.split(',') {
        if value.starts_with("gpu:") {
            let value = value.splitn(3, ':').last().unwrap_or(value);
            let (value, _) = value.split_once('(').unwrap_or((value, ""));

            return value
                .parse()
                .wrap_err_with(|| format!("parsing TRES: {:?})", value));
        }
    }

    Ok(0)
}
