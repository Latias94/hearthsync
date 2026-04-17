mod cli;
pub mod core;

pub use core::error::{AppError, AppResult};

pub fn run_cli() -> AppResult<()> {
    cli::run()
}
