// standard crates
use std::fs;

// internal crates
use crate::app::*;

// external crates
use tempfile::tempdir;

const DEEP: &str = "\
fn f() {
    loop {
        loop {
            loop {}
        }
    }
}
";

#[test]
fn threshold_flag_parses_into_limits() {
    let cli = Cli::parse_from(["lint-imports", "--nestdepth-threshold", "5"]);
    assert_eq!(cli.limits.nestdepth, 5);
}

#[test]
fn threshold_flag_defaults_to_limits_default() {
    let parsed = Cli::parse_from(["lint-imports"]).limits;
    assert_eq!((parsed.nestdepth, Limits::default().nestdepth), (2, 2));
}

#[test]
fn run_nestdepth_check_reports_nesting_over_default_limit() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("deep.rs"), DEEP).unwrap();
    let mut cli = Cli::parse_from(["lint-imports"]);
    cli.path = Some(dir.path().to_path_buf());

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let exit_code = run(&cli, &mut stdout, &mut stderr);
    let stdout = String::from_utf8(stdout).unwrap();

    assert_eq!(exit_code, 1);
    assert!(stdout.contains(":4: nesting depth is 3 (limit 2)"));
    assert!(stdout.contains("or extracting a helper [nestdepth]"));
}

#[test]
fn run_nestdepth_check_skips_assert_paths() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("deep.rs"), DEEP).unwrap();
    let mut cli = Cli::parse_from(["lint-imports"]);
    cli.assert_paths = vec![dir.path().to_path_buf()];

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let exit_code = run(&cli, &mut stdout, &mut stderr);

    assert_eq!(exit_code, 0);
    assert!(stdout.is_empty());
}
