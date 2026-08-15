use std::{fs, path::PathBuf};

use clap::Parser;
use miette::{IntoDiagnostic, Result, miette};

use crate::{config::v1::ShardConfig, instance::resolver::resolve_instances, manifest::ShardManifest};

mod config;
mod error;
mod instance;
mod manifest;

#[derive(Debug, clap::Parser)]
#[command(version, about)]
struct Args {
    /// The path to the shard configuration file.
    #[arg()]
    configuration_file_path: PathBuf,

    /// The path to the Prism Launcher data directory. If unset, a default directory depending on your platform will
    /// be used.
    #[arg(long)]
    prism_data_dir: PathBuf, // TODO: optional
}

fn main() -> Result<()> {
    let args = Args::parse();

    // TODO: validate that the instances directory exists, and probably some others?
    if !args.prism_data_dir.is_dir() {
        return Err(miette!(
            "provided prism data path is either not a directory, or does not exist"
        ));
    }

    let contents = fs::read_to_string(args.configuration_file_path).into_diagnostic()?;
    let config = ShardConfig::from_str(&contents)?;
    let instances = resolve_instances(&config)?;

    let manifest_file = args.prism_data_dir.join("shard-manifest.json");

    let mut manifest = if manifest_file.exists() {
        ShardManifest::from_path(&manifest_file).into_diagnostic()?
    } else {
        ShardManifest::default()
    };

    // TODO: Assign IDs to instances from their inputs sorted alphabetically, e.g. `fabric-26.1`.
    //       How will that work when we add a new layer of inputs? The old instances will kinda be detached then...
    manifest.instance_ids = instances.iter().map(|it| it.name.clone()).collect();
    manifest.write_to_path(&manifest_file).into_diagnostic()?;

    Ok(())
}
