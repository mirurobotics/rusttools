// standard crates
use std::fs;

// internal crates
use crate::app::*;

// external crates
use tempfile::tempdir;

fn cli(path: PathBuf, fix: bool, config: Option<PathBuf>) -> Cli {
    let mut cli = Cli::parse_from(["lint-imports"]);
    cli.path = Some(path);
    cli.fix = fix;
    cli.config = config;
    cli
}

#[test]
fn run_check_mode_returns_zero_for_clean_tree() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("clean.rs"),
        "\
// standard crates
use std::sync::Arc;

fn main() {
    let _ = Arc::new(1);
}
",
    )
    .unwrap();

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let exit_code = run(
        &cli(dir.path().to_path_buf(), false, None),
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(exit_code, 0);
    assert!(
        stdout.is_empty(),
        "unexpected stdout: {:?}",
        String::from_utf8_lossy(&stdout)
    );
    assert!(
        stderr.is_empty(),
        "unexpected stderr: {:?}",
        String::from_utf8_lossy(&stderr)
    );
}

#[test]
fn run_check_mode_reports_violations() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("violations.rs"),
        "use crate::app::state::AppState;\n",
    )
    .unwrap();

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let exit_code = run(
        &cli(dir.path().to_path_buf(), false, None),
        &mut stdout,
        &mut stderr,
    );
    let stdout = String::from_utf8(stdout).unwrap();

    assert_eq!(exit_code, 1);
    assert!(stdout.contains("missing-header"));
    assert!(stdout.contains("violation(s) found."));
    assert!(
        stderr.is_empty(),
        "unexpected stderr: {:?}",
        String::from_utf8_lossy(&stderr)
    );
}

#[test]
fn run_check_mode_reports_multi_anchor_internal_imports() {
    let dir = tempdir().unwrap();
    let config_path = dir.path().join(".lint-imports.toml");
    fs::write(&config_path, "internal_crates = []\n").unwrap();
    fs::write(
        dir.path().join("violations.rs"),
        "\
// internal crates
use crate::{concurrent_cache_tests, single_thread_cache_tests};
",
    )
    .unwrap();

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let exit_code = run(
        &cli(dir.path().to_path_buf(), false, Some(config_path)),
        &mut stdout,
        &mut stderr,
    );
    let stdout = String::from_utf8(stdout).unwrap();

    assert_eq!(exit_code, 1);
    assert!(stdout.contains("multi-anchor-internal-import"));
    assert!(stdout.contains("violation(s) found."));
    assert!(
        stderr.is_empty(),
        "unexpected stderr: {:?}",
        String::from_utf8_lossy(&stderr)
    );
}

#[test]
fn run_fix_mode_rewrites_file_and_reports_count() {
    let dir = tempdir().unwrap();
    let config_path = dir.path().join(".lint-imports.toml");
    fs::write(&config_path, "internal_crates = []\n").unwrap();
    let file_path = dir.path().join("fix.rs");
    fs::write(
        &file_path,
        "\
use crate::filesys::dir::Dir;
use crate::filesys::Overwrite;
",
    )
    .unwrap();

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let exit_code = run(
        &cli(dir.path().to_path_buf(), true, Some(config_path)),
        &mut stdout,
        &mut stderr,
    );
    let stdout = String::from_utf8(stdout).unwrap();
    let fixed = fs::read_to_string(&file_path).unwrap();

    assert_eq!(exit_code, 0);
    assert!(stdout.contains("fixed:"));
    assert!(stdout.contains("1 file(s) fixed."));
    assert!(fixed.contains("use crate::filesys::{Overwrite, dir::Dir};"));
    assert!(
        stderr.is_empty(),
        "unexpected stderr: {:?}",
        String::from_utf8_lossy(&stderr)
    );
}

fn fix_nested_file(nested: &str, file_name: &str, source: &str) -> (i32, String, Vec<u8>, String) {
    let dir = tempdir().unwrap();
    let config_path = dir.path().join(".lint-imports.toml");
    let nested_dir = dir.path().join(nested);
    let file_path = nested_dir.join(file_name);
    fs::create_dir_all(&nested_dir).unwrap();
    fs::write(&config_path, "internal_crates = []\n").unwrap();
    fs::write(&file_path, source).unwrap();

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let exit_code = run_from_dir(
        &nested_dir,
        &cli(PathBuf::from("."), true, Some(config_path)),
        &mut stdout,
        &mut stderr,
    );
    let stdout = String::from_utf8(stdout).unwrap();
    let fixed = fs::read_to_string(&file_path).unwrap();
    (exit_code, stdout, stderr, fixed)
}

#[test]
fn run_fix_mode_rewrites_nested_relative_path() {
    let (exit_code, stdout, stderr, fixed) = fix_nested_file(
        "src/http",
        "deployments.rs",
        "\
// internal crates
use super::errors::HTTPErr;

fn main() {}
",
    );

    assert_eq!(exit_code, 0);
    assert!(stdout.contains("fixed:"));
    assert!(fixed.contains("use crate::http::errors::HTTPErr;"));
    assert!(
        stderr.is_empty(),
        "unexpected stderr: {:?}",
        String::from_utf8_lossy(&stderr)
    );
}

#[test]
fn run_fix_mode_rewrites_nested_relative_mod_file() {
    let (exit_code, stdout, stderr, fixed) = fix_nested_file(
        "src/storage",
        "mod.rs",
        "\
// internal crates
use self::errors::StorageErr;
use self::layout::Layout;

fn main() {}
",
    );

    assert_eq!(exit_code, 0);
    assert!(stdout.contains("fixed:"));
    assert!(fixed.contains("use crate::storage::{errors::StorageErr, layout::Layout};"));
    assert!(
        stderr.is_empty(),
        "unexpected stderr: {:?}",
        String::from_utf8_lossy(&stderr)
    );
}

#[test]
fn run_check_mode_ignores_reexport_shell_mod_rs() {
    let dir = tempdir().unwrap();
    let module_dir = dir.path().join("src/services/deployment");
    let file_path = module_dir.join("mod.rs");
    fs::create_dir_all(&module_dir).unwrap();
    fs::write(
        &file_path,
        "\
mod get;

pub use get::*;
",
    )
    .unwrap();

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let exit_code = run(
        &cli(dir.path().to_path_buf(), false, None),
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(exit_code, 0);
    assert!(
        stdout.is_empty(),
        "unexpected stdout: {:?}",
        String::from_utf8_lossy(&stdout)
    );
    assert!(
        stderr.is_empty(),
        "unexpected stderr: {:?}",
        String::from_utf8_lossy(&stderr)
    );
}

#[test]
fn run_fix_mode_preserves_reexport_shell_mod_rs() {
    let dir = tempdir().unwrap();
    let module_dir = dir.path().join("src/services/deployment");
    let file_path = module_dir.join("mod.rs");
    fs::create_dir_all(&module_dir).unwrap();
    fs::write(
        &file_path,
        "\
mod get;

pub use get::*;
",
    )
    .unwrap();

    let original = fs::read_to_string(&file_path).unwrap();
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let exit_code = run(
        &cli(dir.path().to_path_buf(), true, None),
        &mut stdout,
        &mut stderr,
    );
    let fixed = fs::read_to_string(&file_path).unwrap();
    let stdout = String::from_utf8(stdout).unwrap();

    assert_eq!(exit_code, 0);
    assert_eq!(fixed, original);
    assert!(stdout.contains("0 file(s) fixed."));
    assert!(
        stderr.is_empty(),
        "unexpected stderr: {:?}",
        String::from_utf8_lossy(&stderr)
    );
}

#[test]
fn run_warns_and_continues_when_rs_path_is_unreadable() {
    let dir = tempdir().unwrap();
    fs::create_dir(dir.path().join("broken.rs")).unwrap();
    fs::write(
        dir.path().join("good.rs"),
        "\
// standard crates
use std::fmt::Debug;
",
    )
    .unwrap();

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let exit_code = run(
        &cli(dir.path().to_path_buf(), false, None),
        &mut stdout,
        &mut stderr,
    );
    let stderr = String::from_utf8(stderr).unwrap();

    assert_eq!(exit_code, 0);
    assert!(stderr.contains("warning: could not read"));
}
