// standard crates
use std::fs;

// internal crates
use crate::app::*;

// external crates
use tempfile::tempdir;

fn assert_cli(assert_paths: Vec<PathBuf>) -> Cli {
    Cli {
        path: None,
        fix: false,
        config: None,
        assert_paths,
        assert_threshold: 4,
        funclen_threshold: 50,
    }
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
    Cli {
        path: Some(path),
        fix,
        config: None,
        assert_paths: Vec::new(),
        assert_threshold: 4,
        funclen_threshold,
    }
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
