mod commands;
mod utils;

use commands::CLI;
use std::process::ExitCode;

fn main() -> ExitCode {
    env_logger::init();
    CLI::run()
}
