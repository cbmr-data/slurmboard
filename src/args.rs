use argh::FromArgs;

/// Text-based dashboard for Slurm
#[derive(FromArgs, Debug)]
pub struct Args {
    /// refresh frequency in seconds; a value of zero disables automatic updates
    #[argh(option, default = "5")]
    pub interval: u64,

    /// print version information
    #[argh(switch, short = 'v')]
    pub version: bool,
}
