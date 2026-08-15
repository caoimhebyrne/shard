use std::{collections::BTreeMap, fs, path::PathBuf};

use clap::Parser;
use miette::{IntoDiagnostic, Result};

use crate::{
    config::v1::ShardConfig,
    instance::resolver::resolve_instances,
    manifest::ShardManifest,
    prism::{
        PrismLauncherInstance,
        config::{GENERAL_SECTION, InstanceConfiguration},
        find_prism_data_directory,
        mmc_pack::{self, MultiMcPack, PackComponent},
    },
};

mod config;
mod error;
mod instance;
mod manifest;
mod prism;

#[derive(Debug, clap::Parser)]
#[command(version, about)]
struct Args {
    /// The path to the shard configuration file.
    #[arg()]
    configuration_file_path: PathBuf,

    /// The path to the Prism Launcher data directory. If unset, a default directory depending on your platform will
    /// be used.
    #[arg(long)]
    prism_data_dir: Option<PathBuf>,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let prism_data_dir = find_prism_data_directory(args.prism_data_dir)?;

    println!("using prism data directory: '{}'", prism_data_dir.display());

    let contents = fs::read_to_string(args.configuration_file_path).into_diagnostic()?;
    let config = ShardConfig::from_str(&contents)?;

    let manifest_file = prism_data_dir.join("shard-manifest.json");
    let manifest = if manifest_file.exists() {
        ShardManifest::from_path(&manifest_file).into_diagnostic()?
    } else {
        ShardManifest::default()
    };

    manifest.write_to_path(&manifest_file).into_diagnostic()?;

    let instances = resolve_instances(&config)?;

    for instance in instances {
        let mut configuration = InstanceConfiguration::default();

        configuration.set(GENERAL_SECTION, "ConfigVersion", "1.3");
        configuration.set(GENERAL_SECTION, "iconKey", "default");
        configuration.set(GENERAL_SECTION, "name", instance.name);

        let pack = MultiMcPack {
            format_version: mmc_pack::FORMAT_VERSION,
            components: vec![
                PackComponent {
                    uid: "net.minecraft".into(),
                    important: true,
                    // TODO: A better way to get this?
                    version: Some(instance.source.inputs["minecraft"].clone()),
                    extra: BTreeMap::new(),
                },
                PackComponent {
                    uid: "net.fabricmc.fabric-loader".into(),
                    important: false,
                    version: Some("0.19.3".into()),
                    extra: BTreeMap::new(),
                },
            ],
            extra: BTreeMap::new(),
        };

        let prism_instance = PrismLauncherInstance {
            id: instance.id.clone(),
            configuration,
            pack,
        };

        let instance_directory = prism_data_dir.join("instances").join(&instance.id);
        if instance_directory.exists() {
            println!("instance '{}' already exists, skippping", instance.id);
            continue;
        }

        std::fs::create_dir_all(&instance_directory).into_diagnostic()?;

        let config_file = instance_directory.join("instance.cfg");
        fs::write(config_file, prism_instance.configuration.to_string()).into_diagnostic()?;

        let pack_file = instance_directory.join("mmc-pack.json");
        fs::write(
            pack_file,
            serde_json::to_string(&prism_instance.pack).into_diagnostic()?,
        )
        .into_diagnostic()?;
    }

    Ok(())
}
