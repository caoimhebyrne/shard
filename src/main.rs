use std::{env, fs};

use miette::{IntoDiagnostic, Result, miette};

use crate::config::v1::ShardConfig;

mod config;
mod error;

fn main() -> Result<()> {
    let file_path = env::args()
        .nth(1)
        .ok_or(miette!("expected a file path to be provided as the first argument"))?;

    let contents = fs::read_to_string(file_path).into_diagnostic()?;
    let config = ShardConfig::from_str(&contents)?;
    println!("Config: {config:#?}");

    Ok(())
}
