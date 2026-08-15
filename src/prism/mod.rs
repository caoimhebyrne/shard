use std::{
    env,
    path::{Path, PathBuf},
};

use crate::prism::{config::InstanceConfiguration, mmc_pack::MultiMcPack};

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

    /// The pack associated with this instance.
    pub pack: MultiMcPack,
}

#[derive(Debug, thiserror::Error, miette::Diagnostic)]
#[error("the directory at '{0}' either does not exist, or is not a valid data directory for prism launcher")]
pub struct PrismDataDirectoryError(PathBuf);

/// Returns the first valid path to a Prism Launcher installation.
///
/// # Parameters
/// - `override_path`: The path provided by the user which may, or may not, be a Prism Launcher data directory.
pub fn find_prism_data_directory(override_path: Option<PathBuf>) -> Result<PathBuf, PrismDataDirectoryError> {
    let path = if let Some(value) = override_path {
        value
    } else {
        let home = env::home_dir().expect("user has no home dir?");

        if cfg!(target_os = "windows") {
            home.join("AppData").join("Roaming").join("PrismLauncher")
        } else if cfg!(target_os = "macos") {
            home.join("Library").join("Application Support").join("PrismLauncher")
        } else {
            // TODO: Support the non Flatpak variant: `~/.local/share/PrismLauncher`.
            // If both paths exist, then we need to prompt to figure out which one to use.
            home.join(".var")
                .join("app")
                .join("org.prismlauncher.PrismLauncher")
                .join("data")
                .join("PrismLauncher")
        }
    };

    if !is_valid_prism_data_directory(&path) {
        return Err(PrismDataDirectoryError(path));
    }

    Ok(path)
}

/// Attempts to validate the provided directory as a valid Prism Launcher data directory.
fn is_valid_prism_data_directory(path: &Path) -> bool {
    if !path.is_dir() {
        return false;
    }

    path.join("prismlauncher.cfg").exists()
}
