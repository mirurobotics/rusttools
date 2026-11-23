// standard crates
use std::fs;

// internal crates
use crate::llvmcov::*;

// external crates
use tempfile::tempdir;

const REPORT: &str = r#"{
  "data": [
    {
      "files": [
        {
          "filename": "/repo/src/app/mod.rs",
          "summary": {
            "lines": { "count": 20, "covered": 18, "percent": 90.0 },
            "regions": { "count": 10, "covered": 9, "notcovered": 1, "percent": 90.0 }
          }
        },
        {
          "filename": "/repo/src/parser/mod.rs",
          "summary": {
            "regions": { "count": 4, "covered": 1, "notcovered": 3, "percent": 25.0 }
          }
        }
      ],
      "totals": {}
    }
  ],
  "type": "llvm.coverage.json.export",
  "version": "2.0.1"
}"#;

fn file(filename: &str, count: u64, covered: u64) -> FileCoverage {
    FileCoverage {
        filename: PathBuf::from(filename),
        summary: Summary {
            regions: Regions { count, covered },
        },
    }
}

fn write_covgate(dir: &Path, content: &str) -> PathBuf {
    fs::create_dir_all(dir).unwrap();
    let covgate_file = dir.join(".covgate");
    fs::write(&covgate_file, content).unwrap();
    covgate_file
}

#[test]
fn measure_fails_without_covgate_files() {
    let root = tempdir().unwrap();
    fs::create_dir_all(root.path().join("src/app")).unwrap();
    fs::write(root.path().join("src/app/mod.rs"), "fn main() {}\n").unwrap();

    let error = measure(root.path()).unwrap_err();

    assert!(error.contains("no .covgate files"), "{error}");
}

#[test]
fn measure_fails_on_invalid_threshold_before_running_tests() {
    let root = tempdir().unwrap();
    write_covgate(&root.path().join("src/app"), "abc\n");

    let error = measure(root.path()).unwrap_err();

    assert!(error.contains("could not parse"), "{error}");
}

#[test]
fn measure_fails_when_cargo_llvm_cov_fails() {
    let root = tempdir().unwrap();
    write_covgate(&root.path().join("src/app"), "90\n");

    let error = measure(root.path()).unwrap_err();

    assert!(error.contains("cargo llvm-cov failed"), "{error}");
}

#[test]
fn parse_report_reads_region_summaries() {
    let expected = vec![
        file("/repo/src/app/mod.rs", 10, 9),
        file("/repo/src/parser/mod.rs", 4, 1),
    ];

    assert_eq!(expected, parse_report(REPORT.as_bytes()).unwrap());
}

#[test]
fn parse_report_fails_on_invalid_json() {
    assert!(parse_report(b"not json").is_err());
}

#[test]
fn parse_report_fails_without_coverage_data() {
    assert!(parse_report(br#"{"data": []}"#).is_err());
}

#[test]
fn find_covgate_files_returns_sorted_covgate_paths() {
    let root = tempdir().unwrap();
    let src = root.path().join("src");
    let expected = vec![
        write_covgate(&src, "0\n"),
        write_covgate(&src.join("a"), "90\n"),
        write_covgate(&src.join("a/nested"), "90\n"),
        write_covgate(&src.join("b"), "90\n"),
    ];
    fs::create_dir_all(src.join("c")).unwrap();
    fs::write(src.join("c/mod.rs"), "fn main() {}\n").unwrap();

    assert_eq!(expected, find_covgate_files(&src));
}

#[test]
fn read_module_measures_its_directory() {
    let root = tempdir().unwrap();
    let src = root.path().join("src");
    let covgate_file = src.join("app/.covgate");
    let files = [
        file(&src.join("app/mod.rs").display().to_string(), 10, 9),
        file(&src.join("parser/mod.rs").display().to_string(), 10, 0),
    ];
    let expected = ModuleCoverage {
        name: "app".to_string(),
        covgate_file: covgate_file.clone(),
        threshold: Threshold::Min(90.0),
        coverage: Some(90.0),
    };

    let module = read_module(&src, covgate_file, Threshold::Min(90.0), &files);

    assert_eq!(expected, module);
}

#[test]
fn display_name_is_path_relative_to_src() {
    let src = Path::new("/src");
    let cases = [
        ("src itself", "/src", "src"),
        ("module", "/src/app", "app"),
        ("nested", "/src/rules/funclen", "rules/funclen"),
    ];
    for (name, dir, expected) in cases {
        assert_eq!(expected, display_name(src, Path::new(dir)), "{name}");
    }
}

#[test]
fn coverage_under_sums_regions_under_dir() {
    let dir = Path::new("/src/app");
    let cases = [
        ("no files", vec![file("/src/lib.rs", 4, 4)], None),
        ("empty", vec![file("/src/app/a.rs", 0, 0)], Some(0.0)),
        ("floor", vec![file("/src/app/a.rs", 3, 2)], Some(66.66)),
        ("exact", vec![file("/src/app/a.rs", 100, 29)], Some(29.0)),
        (
            "nested",
            vec![
                file("/src/app/mod.rs", 3, 1),
                file("/src/app/tests/mod.rs", 3, 1),
            ],
            Some(33.33),
        ),
        (
            "sibling prefix",
            vec![
                file("/src/app/mod.rs", 4, 3),
                file("/src/application/mod.rs", 4, 0),
            ],
            Some(75.0),
        ),
    ];
    for (name, files, expected) in cases {
        assert_eq!(expected, coverage_under(dir, &files), "{name}");
    }
}

#[test]
fn read_threshold_parses_first_line() {
    let dir = tempdir().unwrap();
    let cases = [
        ("minimum", "93.33\n", Threshold::Min(93.33)),
        ("maximum", "100\n", Threshold::Min(100.0)),
        ("zero skips", "0\n", Threshold::Skip),
        ("padded", "  85 \nignored\n", Threshold::Min(85.0)),
    ];
    for (name, content, expected) in cases {
        let covgate_file = write_covgate(dir.path(), content);
        let threshold = read_threshold(&covgate_file).unwrap();
        assert_eq!(expected, threshold, "{name}");
    }
}

#[test]
fn read_threshold_fails_on_invalid_content() {
    let dir = tempdir().unwrap();
    for content in ["abc\n", "", "NaN\n", "inf\n", "-1\n", "100.01\n"] {
        let covgate_file = write_covgate(dir.path(), content);
        assert!(read_threshold(&covgate_file).is_err(), "{content:?}");
    }
}

#[test]
fn read_threshold_fails_when_file_is_missing() {
    let dir = tempdir().unwrap();
    let covgate_file = dir.path().join(".covgate");

    assert!(read_threshold(&covgate_file).is_err());
}

#[test]
fn write_threshold_writes_minimum_line() {
    let dir = tempdir().unwrap();
    let covgate_file = dir.path().join(".covgate");

    write_threshold(&covgate_file, 95.5).unwrap();

    assert_eq!("95.5\n", fs::read_to_string(&covgate_file).unwrap());
}

#[test]
fn write_threshold_fails_when_directory_is_missing() {
    let dir = tempdir().unwrap();
    let covgate_file = dir.path().join("missing/.covgate");

    assert!(write_threshold(&covgate_file, 95.5).is_err());
}
