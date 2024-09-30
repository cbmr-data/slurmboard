use std::{fmt, process::Command};

use color_eyre::{
    eyre::{bail, Context},
    Result,
};
use serde::{de, Deserialize, Deserializer};

use super::{misc::format_string, nodes::PartitionName};

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum JobState {
    /// Terminated due to launch failure
    BootFail,
    /// Cancelled by user/admin
    Cancelled,
    /// Completed successfully
    Completed,
    /// Completing; processes may still be running
    Completing,
    /// Waiting for resources to being running
    Configuring,
    /// Terminated due to deadline
    Deadline,
    /// Terminated with non-zero exit code or similar
    Failed,
    /// Terminated due to node failure
    NodeFail,
    OutOfMemory,
    Pending,
    Preempted,
    Requeued,
    RequeueFed,
    RequeueHold,
    Resizing,
    ResvDelHold,
    Revoked,
    Running,
    Signaling,
    SpecialExit,
    StageOut,
    Stopped,
    Suspended,
    Timeout,
}

impl fmt::Display for JobState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&self, f)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Time {
    /// Returned if the duration is invalid, e.g. due to clock skew
    Invalid,
    /// A valid duration; may be inaccurate for suspended jobs
    Duration(JobDuration),
}

impl Time {
    fn parse<'de, D>(value: Option<&str>) -> Result<usize, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = value.ok_or_else(|| de::Error::custom("value not found in time"))?;
        value
            .parse::<usize>()
            .map_err(|_| de::Error::custom(format!("invalid value in TIME: {:?}", value)))
    }
}

impl std::fmt::Display for Time {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Time::Invalid => fmt::Display::fmt("INVALID", f),
            Time::Duration(duration) => write!(f, "{}", duration),
        }
    }
}

/// Represents the time taken by a Slurm job
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct JobDuration {
    days: usize,
    hours: usize,
    minutes: usize,
    seconds: usize,
}

/// Formats the job duration to match squeue output
impl std::fmt::Display for JobDuration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.days > 0 {
            write!(f, "{}-", self.days)?
        }

        if self.days > 0 || self.hours > 0 {
            write!(f, "{:02}:", self.hours)?
        }

        write!(f, "{:02}:{:02}", self.minutes, self.seconds)
    }
}

impl Time {
    fn from_str<'de, D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value: &str = Deserialize::deserialize(deserializer)?;
        if value == "INVALID" {
            return Ok(Time::Invalid);
        }

        let (days, value) = if value.contains('-') {
            let mut values = value.splitn(2, '-');
            let days = Time::parse::<D>(values.next())?;

            let value = values
                .next()
                .ok_or_else(|| de::Error::custom("truncated time; no values after days"))?;

            (days, value)
        } else {
            (0, value)
        };

        let mut values = value.rsplit(':');
        let seconds = Time::parse::<D>(values.next())?;
        let minutes = Time::parse::<D>(values.next())?;
        let hours = Time::parse::<D>(values.next().or(Some("0")))?;

        Ok(Time::Duration(JobDuration {
            days,
            hours,
            minutes,
            seconds,
        }))
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub struct Job {
    /// ID of the job; may be non-unique in `sacct` records
    #[serde(rename = "JOBID")]
    pub id: usize,
    /// Zero or more nodes assigned to this job
    #[serde(deserialize_with = "nodelist_from_str")]
    pub nodelist: Vec<String>,

    /// Name of partition to which this job belongs
    #[serde(deserialize_with = "PartitionName::from_str")]
    pub partition: PartitionName,
    /// State of the job; typically Running since source is `squeue`
    pub state: JobState,
    /// Owner of the job
    pub user: String,

    /// Number of tasks requested by/allocated to the job
    pub tasks: usize,

    /// Number of nodes requested by/allocated to the job (via GRES)
    #[serde(skip_deserializing)]
    pub nodes: usize,
    /// Number of CPUs requested by/allocated to the job (via GRES)
    #[serde(skip_deserializing)]
    pub cpus: usize,
    /// Amount of memory in MBrequested by/allocated to the job (via GRES)
    #[serde(skip_deserializing)]
    pub mem: usize,
    /// Number of GPUs requested by/allocated to the job (via TRES)
    #[serde(skip_deserializing)]
    pub gpus: usize,

    /// Runtime if available
    #[serde(deserialize_with = "Time::from_str")]
    pub time: Time,
    /// Full name of the job
    pub name: String,

    /// Generic resources requested (nodes, cpus, ram)
    #[serde(rename = "TRES_ALLOC")]
    gres: String,
    /// Trackable resources requested (gpus)
    #[serde(rename = "TRES_PER_NODE")]
    tres: String,
}

impl Job {
    pub fn collect(exe: &str) -> Result<Vec<Job>> {
        // FIXME: Generate parameters on demand
        let output = Command::new(exe)
            .args(["--Format", &squeue_format()])
            .output()
            .wrap_err_with(|| format!("failed to execute {:?}", exe))?;

        if !output.status.success() {
            panic!("{:?}", std::str::from_utf8(&output.stderr));
        }

        // TODO: check output.status
        Job::parse(std::io::Cursor::new(output.stdout))
    }

    fn parse<R>(reader: R) -> Result<Vec<Job>>
    where
        R: std::io::Read,
    {
        let mut reader = csv::ReaderBuilder::new()
            .delimiter(b'|')
            .trim(csv::Trim::All)
            .from_reader(reader);

        let mut results = Vec::new();
        for result in reader.deserialize() {
            let mut job: Job = result?;

            // Update GPUs, nodes, CPUs, mem from `tres` and `gres` fields
            job.update_from_gres()?;
            job.update_from_tres()?;

            results.push(job);
        }

        Ok(results)
    }

    fn update_from_gres(&mut self) -> Result<()> {
        if !self.gres.is_empty() {
            for resource in self.gres.split(',') {
                if let Some((key, value)) = resource.split_once('=') {
                    match key {
                        "cpu" => {
                            self.cpus = value
                                .parse()
                                .with_context(|| format!("parsing cpus in GRES: {:?}", self.gres))?
                        }
                        "mem" => {
                            self.mem = parse_memory(value)
                                .with_context(|| format!("parsing mem in GRES: {:?}", self.gres))?;
                        }
                        _ => {}
                    }
                }
            }
        }

        Ok(())
    }

    fn update_from_tres(&mut self) -> Result<()> {
        for resource in self.tres.split(',') {
            let fields: Vec<_> = resource.splitn(3, ':').collect();
            if fields.first() == Some(&"gpu") {
                self.gpus = fields
                    .last()
                    .unwrap()
                    .parse()
                    .with_context(|| format!("parsing gpus in TRES: {:?}", self.tres))?
            }
        }

        Ok(())
    }
}

/// Generates parameter for the `-F` command-line option for `squeue`
fn squeue_format() -> String {
    format_string(
        [
            "JobID",
            "NodeList",
            "Partition",
            "State",
            "UserName",
            "NumTasks",
            "Tres-Alloc",
            "Tres-Per-Node",
            "TimeUsed",
            "Name",
        ]
        .iter(),
    )
}

fn nodelist_from_str<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let value: &str = Deserialize::deserialize(deserializer)?;
    Ok(value
        .split(',')
        .filter(|v| !v.is_empty())
        .map(|v| v.to_string())
        .collect::<Vec<_>>())
}

fn parse_memory(value: &str) -> Result<usize> {
    if value.is_empty() {
        bail!("mem value is empty");
    }

    let mut mem = value[0..value.len() - 1]
        .parse::<f64>()
        .wrap_err("parsing mem")?;

    match &value[value.len() - 1..value.len()] {
        // Slurm supports K in requests, but appears to report M.
        // The unit is therefore handled just in case this changes.
        "K" => mem /= 1024.0,
        "M" => {}
        "G" => mem *= 1024.0,
        "T" => mem *= 1048576.0,
        _ => bail!("invalid mem unit"),
    };

    Ok(mem as usize)
}
