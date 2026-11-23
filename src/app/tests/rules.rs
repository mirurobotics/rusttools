// standard crates
use std::fs;

// internal crates
use crate::app::*;

// external crates
use tempfile::tempdir;

fn assert_cli(assert_paths: Vec<PathBuf>) -> Cli {
    let mut cli = Cli::parse_from(["lint-imports"]);
    cli.assert_paths = assert_paths;
    cli
}

#[test]
fn threshold_flags_parse_into_limits() {
    let cli = Cli::parse_from([
        "lint-imports",
        "--assert-threshold",
        "7",
        "--funclen-threshold",
        "9",
        "--filelen-threshold",
        "11",
    ]);
    let limits = cli.limits;
    assert_eq!(
        (limits.field_assert, limits.funclen, limits.filelen),
        (7, 9, 11)
    );
}

#[test]
fn threshold_flags_default_to_limits_default() {
    let parsed = Cli::parse_from(["lint-imports"]).limits;
    let defaults = Limits::default();
    assert_eq!(
        (parsed.field_assert, parsed.funclen, parsed.filelen),
        (4, 50, 300)
    );
    assert_eq!(
        (defaults.field_assert, defaults.funclen, defaults.filelen),
        (4, 50, 300)
    );
}

#[test]
fn run_assert_check_flags_field_by_field_asserts() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("test.rs"),
        r#"
#[test]
fn test_example() {
    let req = get_request();
    assert_eq!(req.call, Call::GetDeployment);
    assert_eq!(req.method, Method::GET);
    assert_eq!(req.path, "/deployments");
    assert_eq!(req.url, "http://mock/deployments");
}
"#,
    )
    .unwrap();

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let exit_code = run(
        &assert_cli(vec![dir.path().to_path_buf()]),
        &mut stdout,
        &mut stderr,
    );
    let stdout = String::from_utf8(stdout).unwrap();

    assert_eq!(exit_code, 1);
    assert!(stdout.contains("field-by-field-assert"));
    assert!(stdout.contains("`req`"));
}

#[test]
fn run_assert_check_clean_when_below_threshold() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("test.rs"),
        r#"
#[test]
fn test_example() {
    assert_eq!(req.call, Call::GetDeployment);
    assert_eq!(req.method, Method::GET);
}
"#,
    )
    .unwrap();

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let exit_code = run(
        &assert_cli(vec![dir.path().to_path_buf()]),
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(exit_code, 0);
    assert!(stdout.is_empty());
}

#[test]
fn run_assert_check_skipped_when_no_paths() {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let exit_code = run(&assert_cli(vec![]), &mut stdout, &mut stderr);

    assert_eq!(exit_code, 0);
    assert!(stdout.is_empty());
}

fn funclen_cli(path: PathBuf, fix: bool, funclen_threshold: usize) -> Cli {
    let mut cli = Cli::parse_from(["lint-imports"]);
    cli.path = Some(path);
    cli.fix = fix;
    cli.limits.funclen = funclen_threshold;
    cli
}

fn write_over_limit_fn(dir: &Path) {
    fs::write(
        dir.join("long.rs"),
        "\
// standard crates
use std::sync::Arc;

fn long() {
    let _ = Arc::new(1);
    let _ = Arc::new(2);
    let _ = Arc::new(3);
    let _ = Arc::new(4);
}
",
    )
    .unwrap();
}

#[test]
fn run_funclen_check_reports_over_limit_fn() {
    let dir = tempdir().unwrap();
    write_over_limit_fn(dir.path());

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let exit_code = run(
        &funclen_cli(dir.path().to_path_buf(), false, 3),
        &mut stdout,
        &mut stderr,
    );
    let stdout = String::from_utf8(stdout).unwrap();

    assert_eq!(exit_code, 1);
    assert!(stdout.contains("function `long` is 4 lines (limit 3) [funclen]"));
    assert!(stdout.contains("violation(s) found."));
}

#[test]
fn run_funclen_check_fails_in_fix_mode() {
    let dir = tempdir().unwrap();
    write_over_limit_fn(dir.path());

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let exit_code = run(
        &funclen_cli(dir.path().to_path_buf(), true, 3),
        &mut stdout,
        &mut stderr,
    );
    let stdout = String::from_utf8(stdout).unwrap();

    assert_eq!(exit_code, 1);
    assert!(stdout.contains("[funclen]"));
}

#[test]
fn run_funclen_check_clean_under_limit() {
    let dir = tempdir().unwrap();
    write_over_limit_fn(dir.path());

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let exit_code = run(
        &funclen_cli(dir.path().to_path_buf(), false, 50),
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(exit_code, 0);
    assert!(stdout.is_empty());
}

#[test]
fn run_filelen_check_reports_file_over_default_limit() {
    let dir = tempdir().unwrap();
    let source = "const _: u8 = 0;\n".repeat(301);
    fs::write(dir.path().join("long.rs"), source).unwrap();
    let mut cli = Cli::parse_from(["lint-imports"]);
    cli.path = Some(dir.path().to_path_buf());

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let exit_code = run(&cli, &mut stdout, &mut stderr);
    let stdout = String::from_utf8(stdout).unwrap();

    assert_eq!(exit_code, 1);
    assert!(stdout.contains("file is 301 lines (limit 300) [filelen]"));
}

#[test]
fn run_filelen_check_skips_assert_paths() {
    let dir = tempdir().unwrap();
    let source = "const _: u8 = 0;\n".repeat(301);
    fs::write(dir.path().join("long.rs"), source).unwrap();

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let exit_code = run(
        &assert_cli(vec![dir.path().to_path_buf()]),
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(exit_code, 0);
    assert!(stdout.is_empty());
}

#[test]
fn run_check_mode_ignores_reexport_shell_after_allow_marker() {
    let dir = tempdir().unwrap();
    let module_dir = dir.path().join("src/services/deployment");
    fs::create_dir_all(&module_dir).unwrap();
    fs::write(
        module_dir.join("mod.rs"),
        "// lint:allow(filelen)\nmod get;\n\npub use get::*;\n",
    )
    .unwrap();
    let mut cli = Cli::parse_from(["lint-imports"]);
    cli.path = Some(dir.path().to_path_buf());

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let exit_code = run(&cli, &mut stdout, &mut stderr);

    assert_eq!(exit_code, 0);
    assert!(stdout.is_empty());
}

#[test]
fn run_skips_unparseable_rs_file_and_continues() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("broken.rs"),
        "this is not valid rust at all {{{",
    )
    .unwrap();
    fs::write(
        dir.path().join("test.rs"),
        r#"
#[test]
fn test_example() {
    let req = get_request();
    assert_eq!(req.call, Call::GetDeployment);
    assert_eq!(req.method, Method::GET);
    assert_eq!(req.path, "/deployments");
    assert_eq!(req.url, "http://mock/deployments");
}
"#,
    )
    .unwrap();

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let exit_code = run(
        &assert_cli(vec![dir.path().to_path_buf()]),
        &mut stdout,
        &mut stderr,
    );
    let stdout = String::from_utf8(stdout).unwrap();

    assert_eq!(exit_code, 1);
    assert!(stdout.contains("field-by-field-assert"));
    assert!(stdout.contains("\n1 violation(s) found."));
    assert!(!stdout.contains("broken.rs"));
}
