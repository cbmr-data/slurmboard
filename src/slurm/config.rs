use color_eyre::eyre::Context;
use color_eyre::Result;
use core::panic;
use std::collections::HashMap;
use std::process::Command;

use crate::utilities::split_first;

/// Default allocations via DefMemPer* options
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum DefaultMem {
    /// No default allocation
    #[default]
    Unlimited,
    // Default memory allocation per allocated CPU
    PerCPU(usize),
    // Default memory allocation per allocated GPU
    PerGPU(usize),
    // Default memory allocation per allocated node
    PerNode(usize),
}

#[derive(Default, Debug)]
/// Per partition configuration
pub struct PartitionConfig {
    pub default_mem: DefaultMem,
}

/// Global cluster configuration
#[derive(Default, Debug)]
pub struct SlurmConfig {
    pub default_mem: DefaultMem,
    pub partitions: HashMap<String, PartitionConfig>,
}

impl SlurmConfig {
    /// Returns current slurm configuration return by `scontrol`
    pub fn collect() -> Result<SlurmConfig> {
        let mut config = Self::collect_slurm_config()?;
        config.partitions = Self::collect_partition_config()?;

        Ok(config)
    }

    /// Calls `scontrol show config` and collects relevant values
    fn collect_slurm_config() -> Result<SlurmConfig> {
        let mut config = SlurmConfig::default();

        let output = Command::new("scontrol")
            .args(["show", "config"])
            .output()
            .wrap_err("failed to execute `scontrol show config`")?;

        for line in output.stdout.as_slice().split(|&c| c == b'\n') {
            if let Some((key, value)) = split_first(line, b'=') {
                match key.trim_ascii() {
                    b"DefMemPerCPU" | b"DefMemPerGPU" | b"DefMemPerNode" => {
                        if let Some(limit) = Self::parse_default_mem(key, value) {
                            config.default_mem = limit;
                        }
                    }
                    _ => {}
                }
            }
        }

        Ok(config)
    }

    /// Calls `scontrol show partition` and collects relevant per-partition configuration
    /// The nodes associated with each partition are not collected, as this information is
    /// also collected when querying `sinfo` via `Nodes::collect()`
    fn collect_partition_config() -> Result<HashMap<String, PartitionConfig>> {
        let mut partitions = HashMap::new();

        let output = Command::new("scontrol")
            .args(["show", "partition", "--oneline"])
            .output()
            .wrap_err("failed to execute `scontrol show partition`")?;

        for line in output.stdout.as_slice().split(|&c| c == b'\n') {
            let mut values = HashMap::<&[u8], &[u8]>::new();

            for value in line.split(|&c| c.is_ascii_whitespace()) {
                if let Some((key, value)) = split_first(value, b'=') {
                    values.insert(key.trim_ascii(), value);
                }
            }

            // TODO: log warning if partition could not be found
            if let Some(partition) = values.get(b"PartitionName".as_slice()) {
                let mut config = PartitionConfig::default();

                for key in &[&b"DefMemPerCPU"[..], b"DefMemPerGPU", b"DefMemPerNode"] {
                    if let Some(value) = values
                        .get(key)
                        .and_then(|value| Self::parse_default_mem(key, value))
                    {
                        config.default_mem = value;
                        break;
                    }
                }

                partitions.insert(String::from_utf8(partition.to_vec())?, config);
            }
        }

        Ok(partitions)
    }

    /// Parse default memory allocations, returning None on UNLIMITED or invalid values
    fn parse_default_mem(key: &[u8], value: &[u8]) -> Option<DefaultMem> {
        let pred = match key.trim_ascii() {
            b"DefMemPerCPU" => |v| DefaultMem::PerCPU(v),
            b"DefMemPerGPU" => |v| DefaultMem::PerGPU(v),
            b"DefMemPerNode" => |v| DefaultMem::PerCPU(v),
            key => panic!("unknown key {:?}", key),
        };

        Self::parse_limits(value).map(pred)
    }

    /// Parse numerical limits, e.g. `DefMemPerCPU`, returning none on `UNLIMITED` and invalid values
    /// TODO: Log warning on invalid values
    fn parse_limits(value: &[u8]) -> Option<usize> {
        let value = value.trim_ascii();

        // Invalid values and "UNLIMITED" are silently ignored
        String::from_utf8(value.to_vec())
            .ok()?
            .parse::<usize>()
            .ok()
    }
}
