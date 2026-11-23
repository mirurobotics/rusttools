// standard crates
use std::path::PathBuf;

// internal crates
use crate::covgate::*;

fn module(threshold: Threshold, coverage: Option<f64>) -> ModuleCoverage {
    ModuleCoverage {
        name: "app".to_string(),
        covgate_file: PathBuf::from("src/app/.covgate"),
        threshold,
        coverage,
    }
}

fn check(modules: &[ModuleCoverage]) -> (i32, String) {
    let mut stdout = Vec::new();
    let exit_code = run(modules, &mut stdout);
    (exit_code, String::from_utf8(stdout).unwrap())
}

#[test]
fn run_passes_module_above_its_minimum() {
    let (exit_code, stdout) = check(&[module(Threshold::Min(90.0), Some(95.5))]);

    assert_eq!(0, exit_code);
    assert!(stdout.contains("✅ app: 95.5% (requires 90%)"));
    assert!(stdout.contains("All modules meet minimum coverage requirement"));
}

#[test]
fn run_passes_module_at_its_minimum() {
    let (exit_code, stdout) = check(&[module(Threshold::Min(90.0), Some(90.0))]);

    assert_eq!(0, exit_code);
    assert!(stdout.contains("✅ app: 90% (requires 90%)"));
}

#[test]
fn run_fails_when_any_module_is_below_its_minimum() {
    let modules = [
        module(Threshold::Min(90.0), Some(89.99)),
        module(Threshold::Min(90.0), Some(95.0)),
    ];

    let (exit_code, stdout) = check(&modules);

    assert_eq!(1, exit_code);
    assert!(stdout.contains("❌ app: 89.99% (requires 90%)"));
    assert!(stdout.contains("ERROR: One or more modules are below"));
}

#[test]
fn run_skips_module_with_zero_threshold() {
    let (exit_code, stdout) = check(&[module(Threshold::Skip, Some(10.0))]);

    assert_eq!(0, exit_code);
    assert!(stdout.contains("⏭️  app: skipped (threshold: 0)"));
}

#[test]
fn run_warns_without_failing_when_module_has_no_coverage_data() {
    let (exit_code, stdout) = check(&[module(Threshold::Min(90.0), None)]);

    assert_eq!(0, exit_code);
    assert!(stdout.contains("⚠️  app: no source files found in coverage data"));
}
