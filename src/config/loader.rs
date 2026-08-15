use serde::{Deserialize, de::Visitor};

/// The different kinds of loaders that are available.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoaderKind {
    Fabric,
}

/// The different types of versions that can be described.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoaderVersion {
    /// The latest version for the instance's Minecraft version should be used.
    Latest,

    /// An explicit version should be used.
    Explicit(String),
}

/// A definition of which loader and version to use.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Loader {
    pub kind: LoaderKind,
    pub version: LoaderVersion,
}

impl<'de> Deserialize<'de> for Loader {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(LoaderVisitor)
    }
}

/// Responsible for deserializing a [`Loader`] from its `kind@version` representation.
struct LoaderVisitor;

impl Visitor<'_> for LoaderVisitor {
    type Value = Loader;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a valid loader definition, e.g. `fabric@latest`, `fabric@0.19.3`, or `fabric`")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        // The presence of an @ indicates a potentially explicit version.
        let (raw_kind, version) = if let Some(at_idx) = v.find('@') {
            // There is an `@`. The characters before it are the loader kind, and after are the version.
            let raw_kind = &v[..at_idx];
            let raw_version = &v[at_idx + 1..];

            let version = match raw_version {
                "latest" => LoaderVersion::Latest,
                _ => LoaderVersion::Explicit(raw_version.to_string()),
            };

            (raw_kind, version)
        } else {
            // There is no `@`, so the kind is going to be the entire input, and the version will be the latest.
            (v, LoaderVersion::Latest)
        };

        let kind = match raw_kind {
            "fabric" => LoaderKind::Fabric,
            _ => return Err(E::custom(format!("invalid loader kind: '{raw_kind}'"))),
        };

        Ok(Loader { kind, version })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn expect_loader(string: &str, loader: &Loader) {
        #[derive(Deserialize)]
        struct Container {
            loader: Loader,
        }

        let json = format!(r#"{{ "loader": "{string}" }}"#);

        let parsed = serde_json::from_str::<Container>(&json)
            .expect("failed to parse string")
            .loader;

        assert_eq!(&parsed, loader);
    }

    #[test]
    fn parses_fabric_without_version() {
        expect_loader("fabric", &Loader {
            kind: LoaderKind::Fabric,
            version: LoaderVersion::Latest,
        });
    }

    #[test]
    fn parses_fabric_with_version() {
        expect_loader("fabric@0.19.3", &Loader {
            kind: LoaderKind::Fabric,
            version: LoaderVersion::Explicit("0.19.3".into()),
        });
    }

    #[test]
    fn parses_fabric_with_explicit_latest() {
        expect_loader("fabric@latest", &Loader {
            kind: LoaderKind::Fabric,
            version: LoaderVersion::Latest,
        });
    }
}
