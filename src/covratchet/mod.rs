// standard crates
use std::io::Write;

// internal crates
use crate::llvmcov::{write_threshold, ModuleCoverage, Threshold};

pub fn run(modules: &[ModuleCoverage], stdout: &mut impl Write, stderr: &mut impl Write) -> i32 {
    let _ = writeln!(stdout, "\nUpdating .covgate files (ratchet up only)...\n");
    let mut updated = 0;
    for module in modules {
        let outcome = outcome_of(module);
        if let Outcome::Raised { to, .. } = outcome {
            if let Err(error) = write_threshold(&module.covgate_file, to) {
                let _ = writeln!(stderr, "error: {error}");
                return 1;
            }
            updated += 1;
        }
        write_outcome(stdout, &module.name, &outcome);
    }

    let unchanged = modules.len() - updated;
    let _ = writeln!(stdout, "\nDone. Updated: {updated}, Unchanged: {unchanged}");
    0
}

fn outcome_of(module: &ModuleCoverage) -> Outcome {
    let Threshold::Min(threshold) = module.threshold else {
        return Outcome::Skipped;
    };
    let actual = module.coverage.unwrap_or(0.0);
    if actual > threshold {
        return Outcome::Raised {
            from: threshold,
            to: actual,
        };
    }
    Outcome::Kept { actual, threshold }
}

fn write_outcome(stdout: &mut impl Write, name: &str, outcome: &Outcome) {
    let _ = match outcome {
        Outcome::Skipped => writeln!(stdout, "⏭️  {name}: skipped (threshold: 0)"),
        Outcome::Raised { from, to } => writeln!(stdout, "⬆️  {name}: {from}% → {to}%"),
        Outcome::Kept { actual, threshold } => {
            writeln!(stdout, "─  {name}: {actual}% (threshold: {threshold}%)")
        }
    };
}

enum Outcome {
    Skipped,
    Raised { from: f64, to: f64 },
    Kept { actual: f64, threshold: f64 },
}
