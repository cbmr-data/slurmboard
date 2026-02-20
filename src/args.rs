use argh::FromArgs;

/// Text-based dashboard for Slurm
#[derive(FromArgs, Debug)]
pub struct Args {
    /// value of DefMemPerCPU from /etc/slurm/slurm.conf; 0 to disable
    #[argh(option, default = "15948")]
    pub def_mem_per_cpu: u64,

    /// refresh frequency in seconds; a value of zero disables automatic updates
    #[argh(option, default = "5")]
    pub interval: u64,

    /// print version information
    #[argh(switch, short = 'v')]
    pub version: bool,
}
