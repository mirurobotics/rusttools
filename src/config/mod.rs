// standard crates
use std::path::{Path, PathBuf};

// external crates
use serde::Deserialize;

#[derive(Debug, Default, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub internal_crates: Vec<String>,

    #[serde(default)]
    pub labels: Labels,
}

#[derive(Debug, Deserialize)]
pub struct Labels {
    #[serde(default = "default_standard_label")]
    pub standard: String,
    #[serde(default = "default_internal_label")]
    pub internal: String,
    #[serde(default = "default_external_label")]
    pub external: String,
}

fn default_standard_label() -> String {
    "// standard crates".to_string()
}
fn default_internal_label() -> String {
    "// internal crates".to_string()
}
fn default_external_label() -> String {
    "// external crates".to_string()
}

impl Default for Labels {
    fn default() -> Self {
        Self {
            standard: default_standard_label(),
            internal: default_internal_label(),
            external: default_external_label(),
        }
    }
}

impl Config {
    pub fn from_file(path: &Path) -> Result<Self, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("could not read {}: {e}", path.display()))?;
        toml::from_str(&content).map_err(|e| format!("could not parse {}: {e}", path.display()))
    }

    /// Walk up from `start` looking for `.lint-imports.toml`.
    pub fn find_from(start: &Path) -> Result<Self, String> {
        let start = if start.is_file() {
            start.parent().map(Path::to_path_buf).unwrap_or_default()
        } else {
            start.to_path_buf()
        };

        let mut dir: Option<PathBuf> = Some(std::fs::canonicalize(&start).unwrap_or(start));
        while let Some(d) = dir {
            let candidate = d.join(".lint-imports.toml");
            if candidate.is_file() {
                return Self::from_file(&candidate);
            }
            dir = d.parent().map(Path::to_path_buf);
        }
        Ok(Self::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn default_config_values() {
        let config = Config::default();
        assert!(config.internal_crates.is_empty());
        assert_eq!(config.labels.standard, "// standard crates");
        assert_eq!(config.labels.internal, "// internal crates");
        assert_eq!(config.labels.external, "// external crates");
    }

    #[test]
    fn from_file_parses_toml() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(".lint-imports.toml");
        fs::write(
            &path,
            r#"
internal_crates = ["my_lib", "other_lib"]

[labels]
standard = "// std"
internal = "// internal"
external = "// external"
"#,
        )
        .unwrap();

        let config = Config::from_file(&path).unwrap();
        assert_eq!(config.internal_crates, vec!["my_lib", "other_lib"]);
        assert_eq!(config.labels.standard, "// std");
        assert_eq!(config.labels.internal, "// internal");
        assert_eq!(config.labels.external, "// external");
    }

    #[test]
    fn from_file_missing_returns_read_error() {
        let path = Path::new("/nonexistent/.lint-imports.toml");
        let error = Config::from_file(path).unwrap_err();
        assert!(error.starts_with("could not read /nonexistent/.lint-imports.toml"));
    }

    #[test]
    fn from_file_malformed_returns_parse_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(".lint-imports.toml");
        fs::write(&path, "this is not valid toml [[[").unwrap();

        let error = Config::from_file(&path).unwrap_err();
        let expected = format!("could not parse {}", path.display());
        assert!(error.starts_with(&expected));
    }

    #[test]
    fn find_from_walks_up() {
        let dir = tempfile::tempdir().unwrap();
        let child = dir.path().join("sub").join("deep");
        fs::create_dir_all(&child).unwrap();
        fs::write(
            dir.path().join(".lint-imports.toml"),
            r#"internal_crates = ["found_it"]"#,
        )
        .unwrap();

        let config = Config::find_from(&child).unwrap();
        assert_eq!(config.internal_crates, vec!["found_it"]);
    }

    #[test]
    fn find_from_malformed_ancestor_returns_parse_error() {
        let dir = tempfile::tempdir().unwrap();
        let child = dir.path().join("sub");
        fs::create_dir_all(&child).unwrap();
        let path = dir.path().join(".lint-imports.toml");
        fs::write(&path, "this is not valid toml [[[").unwrap();

        let error = Config::find_from(&child).unwrap_err();
        let expected = format!("could not parse {}", path.display());
        assert!(error.starts_with(&expected));
    }

    #[test]
    fn find_from_no_config_returns_default() {
        let dir = tempfile::tempdir().unwrap();
        let config = Config::find_from(dir.path()).unwrap();
        assert!(config.internal_crates.is_empty());
        assert_eq!(config.labels.standard, "// standard crates");
    }
}
