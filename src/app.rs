use std::rc::Rc;
use std::time::{Duration, Instant};

use color_eyre::Result;

use crate::args::Args;
use crate::slurm::{Partition, Slurm};

#[derive(Debug)]
pub struct App {
    /// Is the application running?
    pub running: bool,
    /// Command-line args
    pub args: Args,
    /// Slurm nodes organized by partition
    pub cluster: Rc<Vec<Partition>>,
    /// Time since last automatic update
    last_update: Instant,
}

impl App {
    /// Constructs a new instance of [`App`].
    pub fn new(args: Args) -> Result<Self> {
        let partitions = Slurm::collect(&args.sinfo, &args.squeue)?;

        Ok(Self {
            args,
            running: true,
            cluster: Rc::new(partitions),
            last_update: Instant::now(),
        })
    }

    /// Handles the tick event of the terminal.
    pub fn tick(&mut self) -> Result<bool> {
        if self.args.interval > 0 {
            self.update(self.args.interval)
        } else {
            Ok(false)
        }
    }

    /// Force update of Slurm state
    pub fn update(&mut self, interval: u64) -> Result<bool> {
        // A minimum refresh rate is enforced to prevent the user just holding `r`
        let update_rate = Duration::from_secs(interval.max(1));
        if self.last_update.elapsed() >= update_rate {
            self.cluster = Rc::new(Slurm::collect(&self.args.sinfo, &self.args.squeue)?);
            self.last_update = Instant::now();

            return Ok(true);
        }

        Ok(false)
    }

    /// Set running to false to quit the application.
    pub fn quit(&mut self) {
        self.running = false;
    }
}
