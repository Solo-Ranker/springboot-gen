mod analyzer;
mod cli;
mod config;
mod engine;
mod features;
mod generators;

use anyhow::Result;
use cli::Cli;

fn main() -> Result<()> {
    Cli::run()
}
