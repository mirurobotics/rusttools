// internal crates
use crate::app::*;
use crate::llvmcov::Threshold;

// external crates
use clap::CommandFactory;
use tempfile::tempdir;

fn below_minimum(_: &Path) -> Result<Vec<ModuleCoverage>, String> {
    Ok(vec![ModuleCoverage {
        name: "app".to_string(),
        covgate_file: PathBuf::from(".covgate"),
        threshold: Threshold::Min(90.0),
        coverage: Some(50.0),
    }])
}

fn run_command(dir: &Path, command: &Command) -> (i32, String, String) {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let exit_code = run_coverage(dir, command, below_minimum, &mut stdout, &mut stderr);
    (
        exit_code,
        String::from_utf8(stdout).unwrap(),
        String::from_utf8(stderr).unwrap(),
    )
}

#[test]
fn subcommands_parse_into_command() {
    let covgate = Cli::parse_from(["lint-imports", "covgate"]).command;
    let covratchet = Cli::parse_from(["lint-imports", "covratchet"]).command;

    assert!(matches!(covgate, Some(Command::Covgate)));
    assert!(matches!(covratchet, Some(Command::Covratchet)));
}

#[test]
fn cli_definition_is_valid() {
    Cli::command().debug_assert();
}

#[test]
fn run_coverage_fails_without_covgate_files() {
    let dir = tempdir().unwrap();

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let exit_code = run_coverage(
        dir.path(),
        &Command::Covgate,
        measure,
        &mut stdout,
        &mut stderr,
    );

    assert_eq!(1, exit_code);
    assert_eq!(
        "Running tests with coverage instrumentation...\n\n",
        String::from_utf8(stdout).unwrap()
    );
    assert!(String::from_utf8(stderr).unwrap().contains("no .covgate files"));
}

#[test]
fn run_coverage_dispatches_covgate() {
    let dir = tempdir().unwrap();

    let (exit_code, stdout, stderr) = run_command(dir.path(), &Command::Covgate);

    assert_eq!(1, exit_code);
    assert!(stdout.contains("❌ app: 50% (requires 90%)"));
    assert!(stderr.is_empty());
}

#[test]
fn run_coverage_dispatches_covratchet() {
    let dir = tempdir().unwrap();

    let (exit_code, stdout, stderr) = run_command(dir.path(), &Command::Covratchet);

    assert_eq!(0, exit_code);
    assert!(stdout.contains("─  app: 50% (threshold: 90%)"));
    assert!(stderr.is_empty());
}
