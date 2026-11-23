// standard crates
use std::io::Write;
use std::path::Path;

// internal crates
use crate::llvmcov::{measure, ModuleCoverage};

pub fn run(root: &Path, stdout: &mut impl Write, stderr: &mut impl Write) -> i32 {
    let _ = writeln!(stdout, "Running tests with coverage instrumentation...\n");
    let modules = match measure(root) {
        Ok(modules) => modules,
        Err(error) => {
            let _ = writeln!(stderr, "\nerror: {error}");
            return 1;
        }
    };

    let _ = writeln!(stdout, "\nUpdating .covgate files (ratchet up only)...\n");
    let mut updated = 0;
    for module in &modules {
        let outcome = outcome_of(module);
        if let Outcome::Raised(coverage) = outcome {
            if let Err(error) = std::fs::write(&module.covgate, format!("{coverage}\n")) {
                let _ = writeln!(
                    stderr,
                    "error: could not write {}: {error}",
                    module.covgate.display()
                );
                return 1;
            }
            updated += 1;
        }
        write_outcome(stdout, module, &outcome);
    }

    let unchanged = modules.len() - updated;
    let _ = writeln!(stdout, "\nDone. Updated: {updated}, Unchanged: {unchanged}");
    0
}

fn outcome_of(module: &ModuleCoverage) -> Outcome {
    if module.is_skipped() {
        return Outcome::Skipped;
    }
    let coverage = module.coverage.unwrap_or(0.0);
    if coverage > module.threshold {
        return Outcome::Raised(coverage);
    }
    Outcome::Kept(coverage)
}

fn write_outcome(stdout: &mut impl Write, module: &ModuleCoverage, outcome: &Outcome) {
    let (name, threshold) = (&module.name, module.threshold);
    let _ = match outcome {
        Outcome::Skipped => writeln!(stdout, "⏭️  {name}: skipped (threshold: 0)"),
        Outcome::Raised(coverage) => writeln!(stdout, "⬆️  {name}: {threshold}% → {coverage}%"),
        Outcome::Kept(coverage) => {
            writeln!(stdout, "─  {name}: {coverage}% (threshold: {threshold}%)")
        }
    };
}

enum Outcome {
    Skipped,
    Raised(f64),
    Kept(f64),
}
