// standard crates
use std::fs;

// internal crates
use crate::app::*;

// external crates
use tempfile::tempdir;

#[test]
fn run_warns_and_uses_default_config_when_config_is_malformed() {
    let dir = tempdir().unwrap();
    let config = dir.path().join(".lint-imports.toml");
    fs::write(&config, "this is not valid toml [[[").unwrap();
    fs::write(
        dir.path().join("labeled.rs"),
        "// internal crates\nuse crate::x::Y;\n",
    )
    .unwrap();
    fs::write(dir.path().join("unlabeled.rs"), "use crate::x::Y;\n").unwrap();
    let mut cli = Cli::parse_from(["lint-imports"]);
    cli.path = Some(dir.path().to_path_buf());
    cli.config = Some(config.clone());

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let exit_code = run(&cli, &mut stdout, &mut stderr);
    let stdout = String::from_utf8(stdout).unwrap();

    assert_eq!(exit_code, 1);
    assert!(stdout.contains("unlabeled.rs:"));
    assert!(stdout.contains("[missing-header]"));
    assert!(stdout.contains("\n1 violation(s) found."));
    let stderr = String::from_utf8(stderr).unwrap();
    let expected = format!("warning: could not parse {}", config.display());
    assert!(stderr.starts_with(&expected));
}
