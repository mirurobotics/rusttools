// standard crates
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

// external crates
use serde::Deserialize;
use walkdir::WalkDir;

pub const SRC_DIR: &str = "src";
const COVGATE_FILE: &str = ".covgate";

pub struct ModuleCoverage {
    pub name: String,
    pub covgate: PathBuf,
    pub threshold: Threshold,
    pub coverage: Option<f64>,
}

#[derive(Clone, Copy)]
pub enum Threshold {
    Skip,
    Min(f64),
}

pub fn measure(root: &Path) -> Result<Vec<ModuleCoverage>, String> {
    let src = root.join(SRC_DIR);
    let covgates = find_covgates(&src);
    if covgates.is_empty() {
        return Err(format!("no {COVGATE_FILE} files under {}", src.display()));
    }
    let files = run_llvm_cov(root)?;
    covgates
        .into_iter()
        .map(|covgate| read_module(&src, covgate, &files))
        .collect()
}

fn run_llvm_cov(root: &Path) -> Result<Vec<FileCoverage>, String> {
    let output = Command::new("cargo")
        .args(["llvm-cov", "--json"])
        .current_dir(root)
        .stderr(Stdio::inherit())
        .output()
        .map_err(|e| format!("could not run cargo llvm-cov: {e}"))?;
    if !output.status.success() {
        return Err(format!("tests failed ({})", output.status));
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

fn find_covgates(src: &Path) -> Vec<PathBuf> {
    let mut covgates: Vec<PathBuf> = WalkDir::new(src)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_file() && entry.file_name() == COVGATE_FILE)
        .map(|entry| entry.into_path())
        .collect();
    covgates.sort();
    covgates
}

fn read_module(
    src: &Path,
    covgate: PathBuf,
    files: &[FileCoverage],
) -> Result<ModuleCoverage, String> {
    let dir = covgate.parent().unwrap_or(src);
    let name = display_name(src, dir);
    let coverage = coverage_under(dir, files);
    let threshold = read_threshold(&covgate)?;
    Ok(ModuleCoverage {
        name,
        covgate,
        threshold,
        coverage,
    })
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
    Some((covered as f64 / count as f64 * 10000.0).floor() / 100.0)
}

fn read_threshold(covgate: &Path) -> Result<Threshold, String> {
    let content = std::fs::read_to_string(covgate)
        .map_err(|e| format!("could not read {}: {e}", covgate.display()))?;
    let first_line = content.lines().next().unwrap_or_default().trim();
    let min = first_line
        .parse::<f64>()
        .map_err(|e| format!("could not parse {}: {e}", covgate.display()))?;
    if min == 0.0 {
        return Ok(Threshold::Skip);
    }
    Ok(Threshold::Min(min))
}

pub fn write_threshold(covgate: &Path, min: f64) -> Result<(), String> {
    std::fs::write(covgate, format!("{min}\n"))
        .map_err(|e| format!("could not write {}: {e}", covgate.display()))
}

#[derive(Deserialize)]
struct Report {
    data: Vec<Export>,
}

#[derive(Deserialize)]
struct Export {
    files: Vec<FileCoverage>,
}

#[derive(Deserialize)]
struct FileCoverage {
    filename: PathBuf,
    summary: Summary,
}

#[derive(Deserialize)]
struct Summary {
    regions: Regions,
}

#[derive(Deserialize)]
struct Regions {
    count: u64,
    covered: u64,
}
