use std::{fs, io, path::PathBuf};

use serde::{Deserialize, Serialize};

/// The shard manifest is stored alongside the Prism Launcher data directory. It is used to keep track of which
/// instances are managed by shard.
#[derive(Debug, Default, Serialize, Deserialize)]
// TODO: Version?
pub struct ShardManifest {
    /// The IDs of the instances managed by shard.
    /// An "instance ID" is the name of the directory of the instance within the Prism Launcher `instances` directory.
    pub instance_ids: Vec<String>,
}

impl ShardManifest {
    /// Attempts to parse a shard manifest file at the provided path.
    pub fn from_path(path: &PathBuf) -> Result<ShardManifest, ShardManifestError> {
        let contents = fs::read_to_string(path)?;
        let manifest = serde_json::from_str::<ShardManifest>(&contents)?;

        Ok(manifest)
    }

    /// Attempts to write out the state of this [`ShardManifest`] to the provided path.
    pub fn write_to_path(&self, path: &PathBuf) -> Result<(), ShardManifestError> {
        let contents = serde_json::to_string(self)?;
        fs::write(path, contents)?;

        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ShardManifestError {
    #[error("failed to read shard manifest file")]
    IO(#[from] io::Error),

    #[error("failed to (de)serialize shard manifest file")]
    Serde(#[from] serde_json::Error),
}
