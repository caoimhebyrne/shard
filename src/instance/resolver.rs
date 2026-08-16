use std::collections::BTreeMap;

use miette::{Result, SourceSpan};

use crate::{
    config::v1::ShardConfig,
    instance::{InstanceSource, ResolvedInstance},
};

#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum InstanceResolverError {
    /// A template was referenced that does not exist.
    #[error("undefined template '{0}'")]
    UndefinedTemplate(String),

    /// An matrix was defined, but no Minecraft version was available in its inputs.
    #[error("matrix using template '{0}' does not define any minecraft versions")]
    MissingMinecraftVersion(String),

    /// A placeholder was used in a string that does not correspond to an input.
    #[error("unknown placeholder found in template string")]
    #[diagnostic(help("available inputs: {available}"))]
    UnknownPlaceholder {
        #[source_code]
        template: String,

        #[label(collection, "unknown placeholder")]
        unknown: Vec<SourceSpan>,

        /// The inputs that are available for use, concatenated into a comma seperated string.
        available: String,
    },

    /// A placeholder was opened but never closed.
    #[error("unterminated placeholder in template string")]
    #[diagnostic(help("every '{{' must have a matching '}}'"))]
    UnterminatedPlaceholder {
        #[source_code]
        template: String,

        #[label("this is never closed")]
        placeholder: SourceSpan,
    },
}

pub fn resolve_instances(config: &ShardConfig) -> Result<Vec<ResolvedInstance>> {
    let mut instances = vec![];

    for matrix in &config.matrixes {
        let template = config
            .templates
            .get(&matrix.uses)
            .ok_or(InstanceResolverError::UndefinedTemplate(matrix.uses.clone()))?;

        for inputs in expand_inputs(&matrix.with) {
            // TODO: Maybe make `Inputs` a struct which has `minecraft`, and then a map of extra pairs?
            let minecraft_version = inputs
                .get("minecraft")
                .ok_or_else(|| InstanceResolverError::MissingMinecraftVersion(matrix.uses.clone()))?;

            let hyphenated_values = inputs
                .values()
                .cloned()
                .collect::<Vec<_>>()
                .join("-")
                .chars()
                .map(|c| if c.is_alphanumeric() { c } else { '_' })
                .collect::<String>();

            instances.push(ResolvedInstance {
                id: format!("{}-{}", matrix.uses, hyphenated_values),
                name: interpolate_string(&template.name, &inputs)?,
                loader: template.loader.clone(),
                minecraft_version: minecraft_version.clone(),
                mods: template.mods.clone(),
                source: InstanceSource {
                    inputs,
                    template: matrix.uses.clone(),
                },
            });
        }
    }

    Ok(instances)
}

/// Expands a map of inputs from a matrix into the possible combinations of it.
fn expand_inputs(with: &BTreeMap<String, Vec<String>>) -> Vec<BTreeMap<String, String>> {
    let mut combinations = vec![BTreeMap::new()];

    for (key, values) in with {
        let mut next = Vec::with_capacity(combinations.len() * values.len());

        for combination in &combinations {
            for value in values {
                let mut combination = combination.clone();
                combination.insert(key.clone(), value.clone());
                next.push(combination);
            }
        }

        combinations = next;
    }

    combinations
}

/// Replaces any template placeholders in the provided string using the inputs.
fn interpolate_string(template: &str, inputs: &BTreeMap<String, String>) -> Result<String, InstanceResolverError> {
    let mut result = String::with_capacity(template.len());
    let mut cursor = 0;
    let mut unknown_inputs: Vec<SourceSpan> = vec![];

    while let Some(open_brace_pos) = template[cursor..].find('{') {
        // We can push from the start of the cursor to the open brace (not inclusive), as those characters are not part
        // of the template expansion
        let open_brace_idx = cursor + open_brace_pos;
        result.push_str(&template[cursor..open_brace_idx]);

        // We must be able to find the position of the closing brace relative to the current opening brace.
        let Some(close_brace_pos) = template[open_brace_idx..].find('}') else {
            return Err(InstanceResolverError::UnterminatedPlaceholder {
                template: template.to_string(),
                placeholder: (open_brace_idx, 1).into(),
            });
        };

        // The characters between the opening brace and the closing brace will be the name of the input.
        let close_brace_idx = open_brace_idx + close_brace_pos;
        let input_key = &template[open_brace_idx + 1..close_brace_idx];

        // If an input exists with the provided key, then we can add its value to the string. Otherwise, we collect it
        // into a list of unknown input keys, which will be reported at the end.
        match inputs.get(input_key) {
            Some(value) => result.push_str(value),
            None => unknown_inputs.push((open_brace_idx, close_brace_idx + 1 - open_brace_idx).into()),
        }

        cursor = close_brace_idx + 1;
    }

    if !unknown_inputs.is_empty() {
        return Err(InstanceResolverError::UnknownPlaceholder {
            template: template.to_string(),
            unknown: unknown_inputs,
            available: inputs.keys().cloned().collect::<Vec<_>>().join(", "),
        });
    }

    // There might be some characters after we have finished expanding the template that we should add to the string.
    result.push_str(&template[cursor..]);
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expands_every_combination_of_two_axes() {
        let combinations = expand_inputs(&axes(&[("minecraft", &["26.1", "26.2"]), ("my-env", &["abc", "def"])]));

        assert_eq!(describe(&combinations), [
            "minecraft=26.1, my-env=abc",
            "minecraft=26.1, my-env=def",
            "minecraft=26.2, my-env=abc",
            "minecraft=26.2, my-env=def",
        ]);
    }

    #[test]
    fn expands_no_axes_into_a_single_empty_combination() {
        // A template that takes no inputs still produces exactly one instance.
        let combinations = expand_inputs(&axes(&[]));

        assert_eq!(describe(&combinations), [""]);
    }

    #[test]
    fn expands_an_empty_axis_into_no_combinations() {
        // An input with no values multiplies the combination count by zero, so the whole matrix silently produces
        // nothing. This should eventually be rejected during validation.
        let combinations = expand_inputs(&axes(&[("minecraft", &[])]));

        assert_eq!(describe(&combinations), Vec::<String>::new());
    }

    #[test]
    fn interpolates_placeholders_from_inputs() {
        let name = interpolate_string(
            "{minecraft}-{my-env}",
            &inputs(&[("minecraft", "26.1"), ("my-env", "abc")]),
        )
        .expect("placeholders should resolve");

        assert_eq!(name, "26.1-abc");
    }

    #[test]
    fn interpolates_text_surrounding_placeholders() {
        let name = interpolate_string("prism-{minecraft}-instance", &inputs(&[("minecraft", "26.1")]))
            .expect("placeholders should resolve");

        assert_eq!(name, "prism-26.1-instance");
    }

    #[test]
    fn reports_every_unknown_placeholder_at_once() {
        let error = interpolate_string("{minecraft}-{typo}-{oops}", &inputs(&[("minecraft", "26.1")]))
            .expect_err("unknown placeholders should fail to interpolate");

        let InstanceResolverError::UnknownPlaceholder { unknown, available, .. } = &error else {
            panic!("expected an unknown placeholder error, got {error:?}");
        };

        // Both typos are reported together, rather than only the first one encountered.
        assert_eq!(unknown, &[SourceSpan::from((12, 6)), SourceSpan::from((19, 6))]);
        assert_eq!(available, "minecraft");
    }

    #[test]
    fn reports_an_unterminated_placeholder() {
        let error = interpolate_string("prism-{minecraft", &inputs(&[("minecraft", "26.1")]))
            .expect_err("an unclosed brace should fail to interpolate");

        let InstanceResolverError::UnterminatedPlaceholder { placeholder, .. } = &error else {
            panic!("expected an unterminated placeholder error, got {error:?}");
        };

        assert_eq!(placeholder, &SourceSpan::from((6, 1)));
    }

    #[test]
    fn fails_when_a_matrix_name_uses_an_unknown_placeholder() {
        let config = config(
            "
            version: 1
            templates:
              fabric:
                name: \"{minecraft}-{typo}\"
            matrixes:
              - uses: fabric
                with:
                  minecraft: [\"26.1\"]
            ",
        );

        let error = resolve_instances(&config).expect_err("an unknown placeholder should fail to resolve");

        assert_eq!(error.to_string(), "unknown placeholder found in template string");
    }

    #[test]
    fn resolves_one_instance_per_combination() {
        let config = config(
            "
            version: 1
            templates:
              fabric:
                name: \"{minecraft}-{my-env}\"
            matrixes:
              - uses: fabric
                with:
                  minecraft: [\"26.1\", \"26.2\"]
                  my-env: [\"abc\", \"def\"]
            ",
        );

        let instances = resolve_instances(&config).expect("instances should resolve");

        assert_eq!(names(&instances), ["26.1-abc", "26.1-def", "26.2-abc", "26.2-def"]);
    }

    #[test]
    fn records_the_template_and_inputs_that_produced_an_instance() {
        let config = config(
            "
            version: 1
            templates:
              fabric:
                name: \"{minecraft}\"
            matrixes:
              - uses: fabric
                with:
                  minecraft: [\"26.1\"]
            ",
        );

        let instances = resolve_instances(&config).expect("instances should resolve");
        let [instance] = &instances[..] else {
            panic!("expected exactly one instance, got {}", instances.len());
        };

        assert_eq!(instance.source.template, "fabric");
        assert_eq!(instance.source.inputs, inputs(&[("minecraft", "26.1")]));
    }

    #[test]
    fn fails_when_a_matrix_uses_an_undefined_template() {
        let config = config(
            "
            version: 1
            matrixes:
              - uses: fabric
            ",
        );

        let error = resolve_instances(&config).expect_err("an undefined template should fail to resolve");

        assert_eq!(error.to_string(), "undefined template 'fabric'");
    }

    #[test]
    fn generates_duplicate_names_when_the_name_omits_an_axis() {
        // TODO: `my-env` is not part of the name, so both of its values end up with an instance of the same name.
        //       Duplicate instance names should be rejected.
        let config = config(
            "
            version: 1
            templates:
              fabric:
                name: \"{minecraft}\"
            matrixes:
              - uses: fabric
                with:
                  minecraft: [\"26.1\"]
                  my-env: [\"abc\", \"def\"]
            ",
        );

        let instances = resolve_instances(&config).expect("instances should resolve");

        assert_eq!(names(&instances), ["26.1", "26.1"]);
    }

    /// Builds the `with` map of a matrix from a list of `(input, values)` pairs.
    fn axes(pairs: &[(&str, &[&str])]) -> BTreeMap<String, Vec<String>> {
        pairs
            .iter()
            .map(|(key, values)| ((*key).to_string(), values.iter().map(ToString::to_string).collect()))
            .collect()
    }

    /// Builds the inputs of a single combination from a list of `(input, value)` pairs.
    fn inputs(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
            .collect()
    }

    /// Renders combinations as `"key=value, key=value"` strings, so that assertions stay readable.
    fn describe(combinations: &[BTreeMap<String, String>]) -> Vec<String> {
        combinations
            .iter()
            .map(|combination| {
                combination
                    .iter()
                    .map(|(key, value)| format!("{key}={value}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            })
            .collect()
    }

    /// Parses a config, panicking if it is malformed.
    fn config(yaml: &str) -> ShardConfig {
        ShardConfig::from_str(yaml).expect("test config should be valid")
    }

    /// Collects the names of the provided instances.
    fn names(instances: &[ResolvedInstance]) -> Vec<String> {
        instances.iter().map(|instance| instance.name.clone()).collect()
    }
}
