// standard crates
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

// external crates
use serde::Deserialize;
use walkdir::WalkDir;

pub const SRC_DIR: &str = "src";
const COVGATE_FILE: &str = ".covgate";

#[derive(Debug, PartialEq)]
pub struct ModuleCoverage {
    pub name: String,
    pub covgate_file: PathBuf,
    pub threshold: Threshold,
    pub coverage: Option<f64>,
}

#[derive(Debug, PartialEq)]
pub enum Threshold {
    Skip,
    Min(f64),
}

pub fn measure(root: &Path) -> Result<Vec<ModuleCoverage>, String> {
    let src = root.join(SRC_DIR);
    let covgate_files = find_covgate_files(&src);
    if covgate_files.is_empty() {
        return Err(format!("no {COVGATE_FILE} files under {}", src.display()));
    }
    let thresholds = covgate_files
        .into_iter()
        .map(|path| read_threshold(&path).map(|threshold| (path, threshold)))
        .collect::<Result<Vec<_>, _>>()?;
    let files = run_llvm_cov(root)?;
    let modules: Vec<ModuleCoverage> = thresholds
        .into_iter()
        .map(|(path, threshold)| module_coverage(&src, path, threshold, &files))
        .collect();
    Ok(modules)
}

fn find_covgate_files(src: &Path) -> Vec<PathBuf> {
    let mut covgate_files: Vec<PathBuf> = WalkDir::new(src)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_file() && entry.file_name() == COVGATE_FILE)
        .map(|entry| entry.into_path())
        .collect();
    covgate_files.sort();
    covgate_files
}

fn read_threshold(covgate_file: &Path) -> Result<Threshold, String> {
    let content = std::fs::read_to_string(covgate_file)
        .map_err(|e| format!("could not read {}: {e}", covgate_file.display()))?;
    let first_line = content.lines().next().unwrap_or_default().trim();
    let minimum = first_line
        .parse::<f64>()
        .map_err(|e| format!("could not parse {}: {e}", covgate_file.display()))?;
    if !(0.0..=100.0).contains(&minimum) {
        return Err(format!(
            "{} must be between 0 and 100",
            covgate_file.display()
        ));
    }
    if minimum == 0.0 {
        return Ok(Threshold::Skip);
    }
    Ok(Threshold::Min(minimum))
}

fn run_llvm_cov(root: &Path) -> Result<Vec<FileCoverage>, String> {
    let output = Command::new("cargo")
        .args(["llvm-cov", "--json", "--manifest-path"])
        .arg(root.join("Cargo.toml"))
        .current_dir(root)
        .stderr(Stdio::inherit())
        .output()
        .map_err(|e| format!("could not run cargo llvm-cov: {e}"))?;
    if !output.status.success() {
        return Err(format!("cargo llvm-cov failed ({})", output.status));
    }
    parse_report(&output.stdout)
}

fn parse_report(json: &[u8]) -> Result<Vec<FileCoverage>, String> {
    let report: Report = serde_json::from_slice(json)
        .map_err(|e| format!("could not parse cargo llvm-cov output: {e}"))?;
    report
        .data
        .into_iter()
        .next()
        .map(|export| export.files)
        .ok_or_else(|| "cargo llvm-cov output has no coverage data".to_string())
}

fn module_coverage(
    src: &Path,
    covgate_file: PathBuf,
    threshold: Threshold,
    files: &[FileCoverage],
) -> ModuleCoverage {
    let dir = covgate_file.parent().unwrap_or(src);
    ModuleCoverage {
        name: display_name(src, dir),
        coverage: coverage_under(dir, files),
        covgate_file,
        threshold,
    }
}

fn display_name(src: &Path, dir: &Path) -> String {
    let relative = dir.strip_prefix(src).unwrap_or(dir);
    if relative.as_os_str().is_empty() {
        return SRC_DIR.to_string();
    }
    relative.display().to_string()
}

fn coverage_under(dir: &Path, files: &[FileCoverage]) -> Option<f64> {
    let regions: Vec<&Regions> = files
        .iter()
        .filter(|file| file.filename.starts_with(dir))
        .map(|file| &file.summary.regions)
        .collect();
    if regions.is_empty() {
        return None;
    }
    let count: u64 = regions.iter().map(|r| r.count).sum();
    let covered: u64 = regions.iter().map(|r| r.covered).sum();
    if count == 0 {
        return Some(0.0);
    }
    Some((covered * 10000 / count) as f64 / 100.0)
}

pub fn write_threshold(covgate_file: &Path, minimum: f64) -> Result<(), String> {
    std::fs::write(covgate_file, format!("{minimum}\n"))
        .map_err(|e| format!("could not write {}: {e}", covgate_file.display()))
}

#[derive(Deserialize)]
struct Report {
    data: Vec<Export>,
}

#[derive(Deserialize)]
struct Export {
    files: Vec<FileCoverage>,
}

#[derive(Debug, PartialEq, Deserialize)]
struct FileCoverage {
    filename: PathBuf,
    summary: Summary,
}

#[derive(Debug, PartialEq, Deserialize)]
struct Summary {
    regions: Regions,
}

#[derive(Debug, PartialEq, Deserialize)]
struct Regions {
    count: u64,
    covered: u64,
}

#[cfg(test)]
mod tests;
