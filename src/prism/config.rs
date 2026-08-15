use std::fmt::{self, Display, Formatter};

use ini::{EscapePolicy, Ini, LineSeparator, ParseOption, WriteOption};
use miette::Result;

use crate::error::ini::IniError;

/// The section that Prism Launcher stores instance settings under.
pub const GENERAL_SECTION: &str = "General";

/// Prism Launcher applies its own escaping when it writes `instance.cfg`.
const PARSE_OPTION: ParseOption = ParseOption {
    // Backslashes and quotes are read as ordinary characters rather than being unescaped a second time.
    enabled_quote: false,
    enabled_escape: false,
    enabled_indented_mutiline_value: false,
    enabled_preserve_key_leading_whitespace: false,
};

const WRITE_OPTION: WriteOption = WriteOption {
    // Prism has custom escaping.
    escape_policy: EscapePolicy::Nothing,
    line_separator: LineSeparator::SystemDefault,
    kv_separator: "=",
};

/// The configuration for a Prism Launcher instance, this is parsed from the `instance.cfg` file within the instance
/// directory.
///
/// Prism Launcher writes many more keys than we care about, so the whole ini structure is retained instead of a fixed
/// set of fieldds.
#[derive(Debug, Clone, Default)]
pub struct InstanceConfiguration {
    /// The parsed file.
    ini: Ini,
}

impl InstanceConfiguration {
    /// Attempts to parse an [`InstanceConfiguration`] from the contents of an `instance.cfg` file.
    ///
    /// If the contents are not valid INI, an [`Err`] will be returned.
    pub fn from_str(string: &str) -> Result<InstanceConfiguration> {
        let ini = Ini::load_from_str_opt(string, PARSE_OPTION).map_err(|e| IniError::from(string, e))?;
        Ok(InstanceConfiguration { ini })
    }

    /// Returns the value of a key in the provided section, if it is set.
    pub fn get(&self, section: &str, key: &str) -> Option<&str> {
        self.ini.get_from(Some(section), key)
    }

    /// Sets a key in the provided section, replacing any value that it already had. The section is appended to the
    /// document if it does not exist yet.
    pub fn set(&mut self, section: &str, key: impl Into<String>, value: impl Into<String>) {
        let key = key.into();
        let value = value.into();

        // `Ini::set_to` moves a key that already exists to the end of its section, which would reorder a file that
        // Prism wrote, so an existing value is replaced where it already sits instead.
        //
        // This ensures that the diff between the proposed changes is as clear as possible.
        let existing = self
            .ini
            .section_mut(Some(section))
            .and_then(|properties| properties.iter_mut().find(|(name, _)| *name == key));

        if let Some((_, existing)) = existing {
            *existing = value;
            return;
        }

        self.ini.set_to(Some(section), key, value);
    }

    /// Removes a key from the provided section, returning its value if it was set.
    pub fn remove(&mut self, section: &str, key: &str) -> Option<String> {
        self.ini.delete_from(Some(section), key)
    }
}

impl Display for InstanceConfiguration {
    /// Renders the configuration back into the contents of an `instance.cfg` file.
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut buffer = Vec::new();

        self.ini
            .write_to_opt(&mut buffer, WRITE_OPTION)
            .map_err(|_| fmt::Error)?;

        f.write_str(&String::from_utf8(buffer).map_err(|_| fmt::Error)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_a_key_from_the_general_section() {
        let configuration = configuration(&["[General]", "name=Fabric 1.21", "iconKey=fabric"]);

        assert_eq!(configuration.get(GENERAL_SECTION, "name"), Some("Fabric 1.21"));
        assert_eq!(configuration.get(GENERAL_SECTION, "iconKey"), Some("fabric"));
        assert_eq!(configuration.get(GENERAL_SECTION, "JavaPath"), None);
    }

    #[test]
    fn handles_windows_paths() {
        // Prism does its own escaping, so a backslash in a value is data and not an escaping indicator. Unescaping it
        // here would corrupt the path when it is written back out.
        let configuration = configuration(&[
            "[General]",
            r"JavaPath=C:\Program Files\Java\jdk-21\bin\javaw.exe",
            r#"notes="quoted""#,
        ]);

        assert_eq!(
            configuration.get(GENERAL_SECTION, "JavaPath"),
            Some(r"C:\Program Files\Java\jdk-21\bin\javaw.exe")
        );
        assert_eq!(configuration.get(GENERAL_SECTION, "notes"), Some(r#""quoted""#));
    }

    #[test]
    fn replaces_an_existing_key_in_place() {
        let mut configuration = configuration(&["[General]", "name=Fabric 1.21", "iconKey=fabric"]);

        configuration.set(GENERAL_SECTION, "name", "Fabric 1.22");

        assert_eq!(
            configuration.to_string(),
            render(&["[General]", "name=Fabric 1.22", "iconKey=fabric"])
        );
    }

    #[test]
    fn appends_a_key_that_was_not_already_set() {
        let mut configuration = configuration(&["[General]", "name=Fabric 1.21"]);

        configuration.set(GENERAL_SECTION, "JvmArgs", "-Xmx4096m");

        assert_eq!(
            configuration.to_string(),
            render(&["[General]", "name=Fabric 1.21", "JvmArgs=-Xmx4096m"])
        );
    }

    #[test]
    fn creates_a_section_that_does_not_exist_yet() {
        let mut configuration = configuration(&["[General]", "name=Fabric 1.21"]);

        configuration.set("Foo", "Bar", "baz");

        assert_eq!(
            configuration.to_string(),
            render(&["[General]", "name=Fabric 1.21", "", "[Foo]", "Bar=baz"])
        );
    }

    #[test]
    fn removes_a_key() {
        let mut configuration = configuration(&["[General]", "name=Fabric 1.21", "JvmArgs=-Xmx4096m"]);

        assert_eq!(
            configuration.remove(GENERAL_SECTION, "JvmArgs"),
            Some("-Xmx4096m".to_string())
        );
        assert_eq!(configuration.remove(GENERAL_SECTION, "JvmArgs"), None);
        assert_eq!(configuration.to_string(), render(&["[General]", "name=Fabric 1.21"]));
    }

    #[test]
    fn reports_the_location_of_malformed_input() {
        let contents = render(&["[General]", "=orphan"]);

        let error = InstanceConfiguration::from_str(&contents).expect_err("a value without a key should fail to parse");

        assert_eq!(error.to_string(), "malformed ini");

        // The parser's own message and position are carried into the report, so it points at the offending line rather
        // than only saying that the file as a whole is malformed.
        let labels: Vec<_> = error.labels().expect("the error should be labelled").collect();
        let [label] = &labels[..] else {
            panic!("expected exactly one label, got {}", labels.len());
        };

        assert_eq!(label.label(), Some("missing key"));
        assert_eq!(label.offset(), render(&["[General]"]).len() + 1);
    }

    /// Joins lines using the same separator that [`WRITE_OPTION`] writes, so that assertions hold on every platform.
    fn render(lines: &[&str]) -> String {
        let separator = WRITE_OPTION.line_separator.as_str();
        lines.join(separator) + separator
    }

    /// Parses a configuration from the provided lines, panicking if it is malformed.
    fn configuration(lines: &[&str]) -> InstanceConfiguration {
        InstanceConfiguration::from_str(&render(lines)).expect("test configuration should be valid")
    }
}
