use std::collections::BTreeMap;

use miette::{Result, miette};
use serde::{Deserialize, Serialize};

use crate::{
    config::{BaseShardConfig, loader::Loader},
    error::yaml_serde::YamlSerdeError,
};

/// A configuration for the shard system.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShardConfig {
    /// The version that this [`ShardConfig`] was defined at.
    pub version: u8,

    /// The templates within this [`ShardConfig`].
    #[serde(default)]
    pub templates: BTreeMap<String, ShardTemplate>,

    /// The matrixes defining the output instances.
    #[serde(default)]
    pub matrixes: Vec<ShardMatrix>,
}

/// A template to be used to generate instances within a matrix.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShardTemplate {
    /// The name that should be applied to the instance once created.
    pub name: String,

    /// The string describing the loader.
    pub loader: Option<Loader>,

    /// The mods that this template requires, a map from key to list of mod descriptors.
    #[serde(default)]
    pub mods: BTreeMap<String, Vec<String>>,

    /// The JVM arguments that should be applied to generated instances.
    #[serde(default)]
    pub jvm_args: Vec<String>,
}

/// An element of the shard matrix, which defines the instances to generate.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShardMatrix {
    /// The name of the template that this element uses.
    pub uses: String,

    /// The inputs passed to the template to be used as an expansion.
    #[serde(default)]
    pub with: BTreeMap<String, Vec<String>>,
}

impl ShardConfig {
    /// Attempts to parse a [`ShardConfig`] from the provided YAML string.
    ///
    /// If an error occurs while parsing the configuration (e.g. an unsupported version, invalid YAML, etc.), an [`Err`]
    /// will be returned.
    pub fn from_str(string: &str) -> Result<ShardConfig> {
        // TODO: In the future, we should be able to call `parse_config` as a standalone function, which will return a
        // struct that implements a `ResolvedConfiguration` trait, or something.
        let base: BaseShardConfig = yaml_serde::from_str(string).map_err(|e| YamlSerdeError::from(string, e))?;
        if base.version != 1 {
            return Err(miette!("unsupported config version: {}", base.version));
        }

        let config: ShardConfig = yaml_serde::from_str(string).map_err(|e| YamlSerdeError::from(string, e))?;
        Ok(config)
    }
}
