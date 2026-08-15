#![allow(dead_code)]

use std::collections::BTreeMap;

use crate::config::loader::Loader;

pub mod resolver;

/// A resolved instance built from a matrix within a [`ShardConfig`].
#[derive(Debug)]
pub struct ResolvedInstance {
    /// A unique ID for this instance.
    /// This is derived from the template, and then a sorted hyphenated list of input values.
    pub id: String,

    /// The literal name of the instance (i.e. with all templates expanded).
    pub name: String,

    /// The loader that the instance should use.
    pub loader: Option<Loader>,

    /// A map of mod provider to mod descriptor.
    pub mods: BTreeMap<String, Vec<String>>,

    /// Information about how the instance was resolved (template, inputs, ...)
    pub source: InstanceSource,
}

/// Information about how an instance was resolved.
#[derive(Debug)]
pub struct InstanceSource {
    /// The name of the template that the instance was derived from.
    pub template: String,

    /// The inputs used to generate the instance.
    pub inputs: BTreeMap<String, String>,
}
