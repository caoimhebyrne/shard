use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::config::BaseShardConfig;

/// A configuration for the shard system.
#[derive(Debug, Serialize, Deserialize)]
pub struct ShardConfig {
    /// The version that this [`ShardConfig`] was defined at.
    pub version: u8,

    /// The templates within this [`ShardConfig`].
    pub templates: BTreeMap<String, ShardTemplate>,

    /// The matrix defining the output instances.
    pub matrix: Vec<ShardElement>,
}

/// A template to be used to generate instances within a matrix.
#[derive(Debug, Serialize, Deserialize)]
pub struct ShardTemplate {
    /// The name that should be applied to the instance once created.
    pub name: String,

    /// The string describing the loader.
    pub loader: String,
}

/// An element of the shard matrix, which defines the instances to generate.
#[derive(Debug, Serialize, Deserialize)]
pub struct ShardElement {
    /// The name of the template that this element uses.
    pub uses: String,
}

impl ShardConfig {
    /// Attempts to parse a [`ShardConfig`] from the provided YAML string.
    ///
    /// If an error occurs while parsing the configuration (e.g. an unsupported version, invalid YAML, etc.), an [`Err`]
    /// will be returned.
    pub fn from_str(string: &str) -> Result<ShardConfig, ()> {
        // TODO: In the future, we should be able to call `parse_config` as a standalone function, which will return a
        // struct that implements a `ResolvedConfiguration` trait, or something.
        let base: BaseShardConfig = yaml_serde::from_str(string).map_err(|_| ())?;
        if base.version != 1 {
            return Err(());
        }

        // TODO: Error handling
        yaml_serde::from_str(string).map_err(|_| ())
    }
}
