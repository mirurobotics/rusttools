// standard crates
use std::io::Write;
use std::path::Path;

// internal crates
use crate::llvmcov::{measure, ModuleCoverage, SRC_DIR};

pub fn run(root: &Path, stdout: &mut impl Write, stderr: &mut impl Write) -> i32 {
    let _ = writeln!(stdout, "Running tests with coverage instrumentation...\n");
    let modules = match measure(root) {
        Ok(modules) => modules,
        Err(error) => {
            let _ = writeln!(stderr, "\nerror: {error}");
            return 1;
        }
    };

    let _ = writeln!(
        stdout,
        "\nChecking per-module coverage (modules discovered by .covgate under {SRC_DIR})...\n"
    );
    let mut has_failures = false;
    for module in &modules {
        let status = status_of(module);
        has_failures |= matches!(status, Status::Below(_));
        write_status(stdout, module, &status);
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
    if module.is_skipped() {
        return Status::Skipped;
    }
    let Some(coverage) = module.coverage else {
        return Status::NoFiles;
    };
    if coverage < module.threshold {
        return Status::Below(coverage);
    }
    Status::Meets(coverage)
}

fn write_status(stdout: &mut impl Write, module: &ModuleCoverage, status: &Status) {
    let (name, threshold) = (&module.name, module.threshold);
    let _ = match status {
        Status::Skipped => writeln!(stdout, "⏭️  {name}: skipped (threshold: 0)"),
        Status::NoFiles => writeln!(stdout, "⚠️  {name}: no source files found in coverage data"),
        Status::Below(coverage) => {
            writeln!(stdout, "❌ {name}: {coverage}% (requires {threshold}%)")
        }
        Status::Meets(coverage) => {
            writeln!(stdout, "✅ {name}: {coverage}% (requires {threshold}%)")
        }
    };
}

enum Status {
    Skipped,
    NoFiles,
    Below(f64),
    Meets(f64),
}
