// standard crates
use std::fs;
use std::path::{Path, PathBuf};

// internal crates
use crate::covratchet::*;

// external crates
use tempfile::tempdir;

fn write_covgate(dir: &Path, content: &str) -> PathBuf {
    let covgate_file = dir.join(".covgate");
    fs::write(&covgate_file, content).unwrap();
    covgate_file
}

fn module(covgate_file: &Path, threshold: Threshold, coverage: Option<f64>) -> ModuleCoverage {
    ModuleCoverage {
        name: "app".to_string(),
        covgate_file: covgate_file.to_path_buf(),
        threshold,
        coverage,
    }
}

fn ratchet(modules: &[ModuleCoverage]) -> (i32, String) {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let exit_code = run(modules, &mut stdout, &mut stderr);
    (exit_code, String::from_utf8(stdout).unwrap())
}

#[test]
fn run_raises_minimum_below_coverage() {
    let dir = tempdir().unwrap();
    let covgate_file = write_covgate(dir.path(), "90\n");
    let modules = [module(&covgate_file, Threshold::Min(90.0), Some(95.5))];

    let (exit_code, stdout) = ratchet(&modules);

    assert_eq!(0, exit_code);
    assert_eq!("95.5\n", fs::read_to_string(&covgate_file).unwrap());
    assert!(stdout.contains("⬆️  app: 90% → 95.5%"));
    assert!(stdout.contains("Done. Updated: 1, Unchanged: 0"));
}

#[test]
fn run_keeps_minimum_above_coverage() {
    let dir = tempdir().unwrap();
    let covgate_file = write_covgate(dir.path(), "90\n");
    let modules = [module(&covgate_file, Threshold::Min(90.0), Some(85.0))];

    let (exit_code, stdout) = ratchet(&modules);

    assert_eq!(0, exit_code);
    assert_eq!("90\n", fs::read_to_string(&covgate_file).unwrap());
    assert!(stdout.contains("─  app: 85% (threshold: 90%)"));
    assert!(stdout.contains("Done. Updated: 0, Unchanged: 1"));
}

#[test]
fn run_keeps_minimum_equal_to_coverage() {
    let dir = tempdir().unwrap();
    let covgate_file = write_covgate(dir.path(), "90\n");
    let modules = [module(&covgate_file, Threshold::Min(90.0), Some(90.0))];

    let (exit_code, _) = ratchet(&modules);

    assert_eq!(0, exit_code);
    assert_eq!("90\n", fs::read_to_string(&covgate_file).unwrap());
}

#[test]
fn run_keeps_minimum_when_module_has_no_coverage_data() {
    let dir = tempdir().unwrap();
    let covgate_file = write_covgate(dir.path(), "90\n");
    let modules = [module(&covgate_file, Threshold::Min(90.0), None)];

    let (exit_code, stdout) = ratchet(&modules);

    assert_eq!(0, exit_code);
    assert_eq!("90\n", fs::read_to_string(&covgate_file).unwrap());
    assert!(stdout.contains("─  app: 0% (threshold: 90%)"));
}

#[test]
fn run_skips_module_with_zero_threshold() {
    let dir = tempdir().unwrap();
    let covgate_file = write_covgate(dir.path(), "0\n");
    let modules = [module(&covgate_file, Threshold::Skip, Some(50.0))];

    let (exit_code, stdout) = ratchet(&modules);

    assert_eq!(0, exit_code);
    assert_eq!("0\n", fs::read_to_string(&covgate_file).unwrap());
    assert!(stdout.contains("⏭️  app: skipped (threshold: 0)"));
    assert!(stdout.contains("Done. Updated: 0, Unchanged: 1"));
}

#[test]
fn run_fails_when_covgate_file_cannot_be_written() {
    let dir = tempdir().unwrap();
    let covgate_file = dir.path().join("missing/.covgate");
    let modules = [module(&covgate_file, Threshold::Min(90.0), Some(95.5))];

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let exit_code = run(&modules, &mut stdout, &mut stderr);

    assert_eq!(1, exit_code);
    assert!(!stderr.is_empty());
    assert!(!String::from_utf8(stdout).unwrap().contains("Done."));
}
