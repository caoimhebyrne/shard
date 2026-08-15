#![allow(dead_code)]

use crate::prism::config::InstanceConfiguration;

pub mod config;
pub mod mmc_pack;

/// Each instance in Prism Launcher has a seperate directory in the `instances` folder. The name of the directory is the
/// ID.
#[derive(Debug)]
pub struct PrismLauncherInstance {
    /// The ID of the instance.
    pub id: String,

    /// The configuration of this instance.
    pub configuration: InstanceConfiguration,
}
