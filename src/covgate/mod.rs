// standard crates
use std::io::Write;

// internal crates
use crate::llvmcov::{ModuleCoverage, Threshold, SRC_DIR};

pub fn run(modules: &[ModuleCoverage], stdout: &mut impl Write) -> i32 {
    let _ = writeln!(
        stdout,
        "\nChecking per-module coverage (modules discovered by .covgate under {SRC_DIR})...\n"
    );
    let mut has_failures = false;
    for module in modules {
        let status = status_of(module);
        has_failures |= matches!(status, Status::Below { .. });
        write_status(stdout, &module.name, &status);
    }
    let _ = writeln!(stdout);

    if has_failures {
        let _ = writeln!(
            stdout,
            "ERROR: One or more modules are below their minimum coverage threshold"
        );
        return 1;
    }
    let _ = writeln!(stdout, "✅ All modules meet minimum coverage requirement");
    0
}

fn status_of(module: &ModuleCoverage) -> Status {
    let Threshold::Min(required) = module.threshold else {
        return Status::Skipped;
    };
    let Some(actual) = module.coverage else {
        return Status::NoFiles;
    };
    if actual < required {
        return Status::Below { actual, required };
    }
    Status::Meets { actual, required }
}

fn write_status(stdout: &mut impl Write, name: &str, status: &Status) {
    let _ = match status {
        Status::Skipped => writeln!(stdout, "⏭️  {name}: skipped (threshold: 0)"),
        Status::NoFiles => writeln!(stdout, "⚠️  {name}: no source files found in coverage data"),
        Status::Below { actual, required } => {
            writeln!(stdout, "❌ {name}: {actual}% (requires {required}%)")
        }
        Status::Meets { actual, required } => {
            writeln!(stdout, "✅ {name}: {actual}% (requires {required}%)")
        }
    };
}

enum Status {
    Skipped,
    NoFiles,
    Below { actual: f64, required: f64 },
    Meets { actual: f64, required: f64 },
}
