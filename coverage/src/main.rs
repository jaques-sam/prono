use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

const TARGET_DIR: &str = "target/coverage-debug";
const OUTPUT_DIR: &str = "target/coverage";

fn workspace_dir() -> PathBuf {
    let output = std::process::Command::new(env!("CARGO"))
        .arg("locate-project")
        .arg("--workspace")
        .arg("--message-format=plain")
        .output()
        .unwrap()
        .stdout;
    let cargo_path = Path::new(std::str::from_utf8(&output).unwrap().trim());
    cargo_path.parent().unwrap().to_path_buf()
}

fn run_unit_tests(cargo_env: &[(&str, &str)]) {
    println!("=== running unit tests with coverage support ===");

    let cargo_result = Command::new("cargo")
        .envs(cargo_env.iter().map(|(k, v)| (k, v)))
        .arg("nextest")
        .arg("run")
        .arg("--target-dir")
        .arg(TARGET_DIR)
        .arg("--workspace")
        .arg("--all-targets")
        .arg("--all-features")
        .status()
        .expect("Failed to run cargo test command");

    assert!(cargo_result.success(), "Unit tests failed to run");
}

fn run_gcovr(cargo_env: &[(&str, &str)], devmode: bool, work_dir: &Path) {
    let coverage_dir = Path::new(OUTPUT_DIR);
    std::fs::create_dir_all(coverage_dir).expect("failed to create coverage output dir");

    let (fmt, file) = if devmode {
        ("lcov", coverage_dir.join("tests.lcov"))
    } else {
        ("html", coverage_dir.to_path_buf())
    };
    println!("=== creating {fmt} coverage report ===");

    let binary_dir = Path::new(TARGET_DIR).join("debug/deps");
    let grcov_args = vec![
        ".",
        "--source-dir",
        ".",
        "--binary-path",
        binary_dir.to_str().unwrap(),
        "--output-types",
        fmt,
        "--branch",
        "--ignore-not-existing",
        "--ignore",
        "../*",
        "--ignore",
        "\"/*\"",
        "--output-path",
        file.to_str().unwrap(),
        "--log-level",
        "ERROR",
    ];

    let grcov_result = Command::new("grcov")
        .envs(cargo_env.iter().map(|(k, v)| (k, v)))
        .current_dir(work_dir)
        .args(grcov_args)
        .status()
        .expect("Failed to run grcov command");

    assert!(
        grcov_result.success(),
        "grcov command failed with non-zero exit code"
    );
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let devmode = args.iter().any(|arg| arg == "--dev-mode");

    let cargo_env = vec![
        ("CARGO_INCREMENTAL", "0"),
        ("RUSTFLAGS", "-Cinstrument-coverage"),
        ("LLVM_PROFILE_FILE", ".coverage/cargo-test-%p-%m.profraw"),
        ("RUST_BACKTRACE", "1"),
    ];

    let ws_dir = workspace_dir();

    run_unit_tests(&cargo_env);
    run_gcovr(&cargo_env, devmode, &ws_dir);
}
