#![doc = "Command-line boundary for validating, executing, and evaluating worlds."]

mod calibrate;
mod cli;
mod error;
mod files;
mod simulate;

use std::process::ExitCode;

use clap::Parser;
use cli::Cli;

fn main() -> ExitCode {
    match cli::run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}
