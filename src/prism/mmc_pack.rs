use std::{
    collections::BTreeMap,
    fmt::{self, Display, Formatter},
    ops::Not,
};

use serde::{Deserialize, Serialize};
use serde_json::Value;
/// The only `formatVersion` that Prism Launcher accepts.
pub const FORMAT_VERSION: u8 = 1;

/// The component list for a Prism Launcher instance, parsed from the `mmc-pack.json` file within the instance
/// directory.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MultiMcPack {
    /// The version of the file, which is always [`FORMAT_VERSION`].
    pub format_version: u8,

    /// The components that this pack requires.
    pub components: Vec<PackComponent>,

    /// Any other key-value pairs that Prism might have (that we don't care about).
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// A single entry in the component list.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackComponent {
    /// A unique identifier for the component, e.g. `net.minecraft`.
    pub uid: String,

    /// The version. Prism treats this as optional, and omits it when it is empty.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,

    /// Whether the component needs to be resolved for the instance to launch, this should only be set for the
    /// Minecraft component.
    #[serde(default, skip_serializing_if = "<&bool>::not")]
    pub important: bool,

    /// Any other key-value pairs that Prism might have (that we don't care about).
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

impl MultiMcPack {
    /// Attempts to parse a [`MultiMcPack`] from the contents of an `mmc-pack.json` file.
    ///
    /// If the contents are not valid JSON, or a component is missing its `uid`, an [`Err`] will be returned.
    pub fn from_str(string: &str) -> Result<MultiMcPack, serde_json::Error> {
        serde_json::from_str(string)
    }
}

impl Display for MultiMcPack {
    /// Renders the pack back into the contents of an `mmc-pack.json` file.
    ///
    /// The output matches what Prism itself writes, so that the diff between the proposed changes is as clear as
    /// possible.
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut buffer = Vec::new();

        // Prism writes with `QJsonDocument::Indented`, which indents by four spaces rather than the two that
        // `to_string_pretty` uses.
        let formatter = serde_json::ser::PrettyFormatter::with_indent(b"    ");
        let mut serializer = serde_json::Serializer::with_formatter(&mut buffer, formatter);

        // Qt sorts the keys of a `QJsonObject`, so Prism's output is ordered alphabetically rather than in declaration
        // order. Going through `Value` first reproduces that.
        let value = serde_json::to_value(self).map_err(|_| fmt::Error)?;
        value.serialize(&mut serializer).map_err(|_| fmt::Error)?;

        f.write_str(&String::from_utf8(buffer).map_err(|_| fmt::Error)?)?;

        // `QJsonDocument::toJson` terminates the document with a newline.
        f.write_str("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PRISM_OUTPUT: &str = r#"{
    "components": [
        {
            "cachedName": "LWJGL 3",
            "cachedVersion": "3.4.1",
            "cachedVolatile": true,
            "dependencyOnly": true,
            "uid": "org.lwjgl3",
            "version": "3.4.1"
        },
        {
            "cachedName": "Minecraft",
            "cachedRequires": [
                {
                    "suggests": "3.4.1",
                    "uid": "org.lwjgl3"
                }
            ],
            "cachedVersion": "26.2",
            "important": true,
            "uid": "net.minecraft",
            "version": "26.2"
        },
        {
            "cachedName": "Fabric Loader",
            "cachedRequires": [
                {
                    "uid": "net.fabricmc.intermediary"
                }
            ],
            "cachedVersion": "0.19.3",
            "uid": "net.fabricmc.fabric-loader",
            "version": "0.19.3"
        }
    ],
    "formatVersion": 1
}
"#;

    #[test]
    fn reproduces_file_contents_identically() {
        let pack = MultiMcPack::from_str(PRISM_OUTPUT).expect("prism's own output should parse");

        assert_eq!(pack.to_string(), PRISM_OUTPUT);
    }

    #[test]
    fn retains_unknown_component_pairs() {
        let pack = MultiMcPack::from_str(PRISM_OUTPUT).expect("prism's own output should parse");
        let component = &pack.components[0];

        assert_eq!(component.uid, "org.lwjgl3");
        assert_eq!(component.extra["cachedName"], "LWJGL 3");
        assert_eq!(component.extra["dependencyOnly"], true);

        // The modelled fields are not left in the map as well, which would duplicate them on the way out.
        assert!(!component.extra.contains_key("uid"));
        assert!(!component.extra.contains_key("version"));
    }

    #[test]
    fn retains_unknown_top_level_pairs() {
        let pack = MultiMcPack::from_str(r#"{"formatVersion": 1, "components": [], "somethingNew": 42}"#)
            .expect("an unknown top-level key should parse");

        assert_eq!(pack.extra["somethingNew"], 42);
        assert_eq!(
            pack.to_string(),
            render(&[
                "{",
                r#"    "components": [],"#,
                r#"    "formatVersion": 1,"#,
                r#"    "somethingNew": 42"#,
                "}"
            ])
        );
    }

    #[test]
    fn reads_a_component_without_a_version() {
        // Prism omits `version` when it is empty, so a missing key is not an error.
        let pack = MultiMcPack::from_str(r#"{"formatVersion": 1, "components": [{"uid": "net.minecraft"}]}"#)
            .expect("a component without a version should parse");

        assert_eq!(pack.components[0].version, None);
        assert!(!pack.components[0].important);
    }

    #[test]
    fn omits_falsy_fields() {
        let pack = pack(PackComponent {
            uid: "net.minecraft".to_string(),
            version: None,
            important: false,
            extra: BTreeMap::new(),
        });

        assert_eq!(
            pack.to_string(),
            render(&[
                "{",
                r#"    "components": ["#,
                "        {",
                r#"            "uid": "net.minecraft""#,
                "        }",
                "    ],",
                r#"    "formatVersion": 1"#,
                "}",
            ])
        );
    }

    #[test]
    fn writes_basic_minecraft_component() {
        // Prism resolves the dependencies and caches itself, so shard only has to write the critical fields.
        let pack = pack(PackComponent {
            uid: "net.minecraft".to_string(),
            version: Some("26.2".to_string()),
            important: true,
            extra: BTreeMap::new(),
        });

        assert_eq!(
            pack.to_string(),
            render(&[
                "{",
                r#"    "components": ["#,
                "        {",
                r#"            "important": true,"#,
                r#"            "uid": "net.minecraft","#,
                r#"            "version": "26.2""#,
                "        }",
                "    ],",
                r#"    "formatVersion": 1"#,
                "}",
            ])
        );
    }

    #[test]
    fn fails_when_a_component_has_no_uid() {
        // `uid` is the one field that Prism requires of a component.
        MultiMcPack::from_str(r#"{"formatVersion": 1, "components": [{"version": "26.2"}]}"#)
            .expect_err("a component without a uid should fail to parse");
    }

    /// Joins lines the way [`MultiMcPack`] writes them, including the trailing newline.
    fn render(lines: &[&str]) -> String {
        lines.join("\n") + "\n"
    }

    /// Builds a pack containing only the provided component.
    fn pack(component: PackComponent) -> MultiMcPack {
        MultiMcPack {
            format_version: FORMAT_VERSION,
            components: vec![component],
            extra: BTreeMap::new(),
        }
    }
}
