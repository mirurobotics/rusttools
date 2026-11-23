// standard crates
use std::io::Write;
use std::path::{Path, PathBuf};

// internal crates
use crate::checker::check;
use crate::classifier::Classifier;
use crate::config::Config;
use crate::fixer::fix_file;
use crate::parser::parse;
use crate::rules::{check_files, Limits, Rule, ASSERT_PATH_RULES, PATH_RULES};

// external crates
use clap::Parser as ClapParser;
use walkdir::WalkDir;

#[derive(ClapParser)]
#[command(
    name = "lint",
    about = "Lint Rust import grouping, ordering, comment headers, and test assert patterns"
)]
pub struct Cli {
    /// Directory to scan for import checking
    #[arg(long)]
    pub path: Option<PathBuf>,

    /// Auto-fix import violations in place
    #[arg(long)]
    pub fix: bool,

    /// Path to .lint-imports.toml config file
    #[arg(long)]
    pub config: Option<PathBuf>,

    /// Directories to scan for field-by-field assert detection in test files
    #[arg(long = "assert-paths", num_args = 1..)]
    pub assert_paths: Vec<PathBuf>,

    /// Minimum field-asserts on the same receiver to flag (default: 4)
    #[arg(long = "assert-threshold", default_value_t = 4)]
    pub assert_threshold: usize,

    /// Max non-blank, non-comment body lines per function (0 disables)
    #[arg(long = "funclen-threshold", default_value_t = 50)]
    pub funclen_threshold: usize,

    #[arg(
        long = "filelen-threshold",
        default_value_t = 300,
        help = "Max non-blank, non-comment lines per file (0 disables)"
    )]
    pub filelen_threshold: usize,
}

pub fn run(cli: &Cli, stdout: &mut impl Write, stderr: &mut impl Write) -> i32 {
    let base_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    run_from_dir(&base_dir, cli, stdout, stderr)
}

fn run_from_dir(
    base_dir: &Path,
    cli: &Cli,
    stdout: &mut impl Write,
    stderr: &mut impl Write,
) -> i32 {
    let mut totals = Totals::default();
    let path_files: Vec<PathBuf> = cli
        .path
        .iter()
        .flat_map(|path| rust_files(base_dir, path))
        .collect();

    // Run import checking if --path is provided.
    if cli.path.is_some() {
        let config = load_config(base_dir, cli);
        let classifier = Classifier::new(&config);

        for file_path in &path_files {
            process_file(
                file_path,
                cli,
                &classifier,
                &config,
                &mut totals,
                stdout,
                stderr,
            );
        }
    }

    let assert_files: Vec<PathBuf> = cli
        .assert_paths
        .iter()
        .flat_map(|dir| rust_files(base_dir, dir))
        .collect();
    let limits = Limits {
        funclen: cli.funclen_threshold,
        field_assert: cli.assert_threshold,
        filelen: cli.filelen_threshold,
    };
    let mut rule_diagnostics = run_rules(&assert_files, ASSERT_PATH_RULES, &limits, stdout);
    rule_diagnostics += run_rules(&path_files, PATH_RULES, &limits, stdout);

    if cli.fix {
        let _ = writeln!(stdout, "\n{} file(s) fixed.", totals.files_fixed);
        // rules have no auto-fix; their diagnostics still fail the run.
        if rule_diagnostics > 0 {
            return 1;
        }
        return 0;
    }

    let total_issues = totals.diagnostics + rule_diagnostics;
    if total_issues > 0 {
        let _ = writeln!(stdout, "\n{} violation(s) found.", total_issues);
        return 1;
    }

    0
}

fn run_rules(files: &[PathBuf], rules: &[Rule], limits: &Limits, stdout: &mut impl Write) -> usize {
    let diagnostics = check_files(files, rules, limits);
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    for diagnostic in &diagnostics {
        let file = &diagnostic.file;
        let display_path = file.strip_prefix(&cwd).unwrap_or(file).display();
        let _ = writeln!(
            stdout,
            "{}:{}: {} [{}]",
            display_path, diagnostic.line, diagnostic.message, diagnostic.kind
        );
    }
    diagnostics.len()
}

#[derive(Default)]
struct Totals {
    diagnostics: usize,
    files_fixed: usize,
}

fn load_config(base_dir: &Path, cli: &Cli) -> Config {
    match (&cli.config, &cli.path) {
        (Some(cfg), _) => Config::from_file(&resolve_input_path(base_dir, cfg)),
        (None, Some(path)) => Config::find_from(&resolve_input_path(base_dir, path)),
        (None, None) => Config::default(),
    }
}

fn rust_files(base_dir: &Path, path: &Path) -> Vec<PathBuf> {
    let root = resolve_input_path(base_dir, path);
    let mut files: Vec<PathBuf> = WalkDir::new(root)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "rs"))
        .map(|entry| std::fs::canonicalize(entry.path()).unwrap_or_else(|_| entry.into_path()))
        .collect();
    files.sort();
    files
}

fn resolve_input_path(base_dir: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        base_dir.join(path)
    }
}

fn process_file(
    file_path: &Path,
    cli: &Cli,
    classifier: &Classifier,
    config: &Config,
    totals: &mut Totals,
    stdout: &mut impl Write,
    stderr: &mut impl Write,
) {
    let content = match std::fs::read_to_string(file_path) {
        Ok(content) => content,
        Err(error) => {
            let _ = writeln!(
                stderr,
                "warning: could not read {}: {error}",
                file_path.display()
            );
            return;
        }
    };

    let block = parse(&content);
    if should_skip_reexport_shell(file_path, &content, &block) {
        return;
    }

    if cli.fix {
        let fixed = fix_file(file_path, &content, &block, classifier, config);
        if fixed == content {
            return;
        }

        if let Err(error) = std::fs::write(file_path, &fixed) {
            let _ = writeln!(
                stderr,
                "error: could not write {}: {error}",
                file_path.display()
            );
            return;
        }

        totals.files_fixed += 1;
        let _ = writeln!(stdout, "fixed: {}", file_path.display());
        return;
    }

    let diagnostics = check(file_path, &block, classifier, config);
    if diagnostics.is_empty() {
        return;
    }

    for diagnostic in &diagnostics {
        let _ = writeln!(
            stdout,
            "{}:{}: {} [{}]",
            file_path.display(),
            diagnostic.line,
            diagnostic.message,
            diagnostic.kind
        );
    }
    totals.diagnostics += diagnostics.len();
}

fn should_skip_reexport_shell(
    file_path: &Path,
    content: &str,
    block: &crate::parser::ImportBlock,
) -> bool {
    if file_path.file_name().and_then(|name| name.to_str()) != Some("mod.rs") {
        return false;
    }

    let uses = block.use_statements();
    if uses.is_empty()
        || !uses
            .iter()
            .all(|stmt| stmt.attrs.is_empty() && stmt.text.trim_start().starts_with("pub use "))
    {
        return false;
    }

    let lines: Vec<&str> = content.lines().collect();
    let prelude_ok = lines
        .iter()
        .take(block.start_line.saturating_sub(1))
        .all(|line| {
            let trimmed = line.trim();
            trimmed.is_empty() || is_module_declaration(trimmed)
        });
    let trailing_ok = lines
        .iter()
        .skip(block.end_line.saturating_sub(1))
        .all(|line| line.trim().is_empty());

    prelude_ok && trailing_ok
}

fn is_module_declaration(trimmed: &str) -> bool {
    let Some(stmt) = trimmed.strip_suffix(';') else {
        return false;
    };

    let parts: Vec<&str> = stmt.split_whitespace().collect();
    match parts.as_slice() {
        ["mod", _] => true,
        [visibility, "mod", _] => visibility.starts_with("pub"),
        _ => false,
    }
}

#[cfg(test)]
mod tests;
