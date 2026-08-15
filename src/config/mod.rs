use serde::Deserialize;

pub mod loader;
pub mod v1;

/// A basic "minimal" Shard config that can be used to load a specialised variant.
///
/// The configuration file is typically parsed into this first, so that we can understand the version of the config,
/// and then we can safely attempt to parse the correct variant.
#[derive(Debug, Deserialize)]
struct BaseShardConfig {
    /// The version that this [`BaseShardConfig`] was defined at.
    version: u8,
}
