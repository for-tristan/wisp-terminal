use std::path::PathBuf;

use clap::Parser;

#[derive(Debug, Parser)]
#[command(
    name = "wisp",
    version,
    about = "A high-performance, GPU-accelerated terminal emulator written in Rust"
)]
pub struct Cli {
    #[arg(short, long, value_name = "FILE")]
    pub config: Option<PathBuf>,

    #[arg(short, long, value_name = "DIR")]
    pub working_dir: Option<PathBuf>,

    #[arg(long)]
    pub print_config_path: bool,

    #[arg(short, long)]
    pub verbose: bool,
}

impl Cli {
    pub fn log_level(&self) -> &'static str {
        if self.verbose {
            "debug"
        } else {
            "info"
        }
    }
}
