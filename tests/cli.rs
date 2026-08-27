use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT_TEMP_DIR: AtomicU64 = AtomicU64::new(0);

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        let sequence = NEXT_TEMP_DIR.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("boa-cli-tests-{}-{sequence}", std::process::id()));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

fn boa(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_boa"))
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn cli_converts_and_refines_with_both_algorithms() {
    let temp_dir = TempDir::new();
    let text_path = temp_dir.0.join("graph.boa.txt");
    let binary_path = temp_dir.0.join("graph.boa");
    fs::write(&text_path, b"List[0]{@0}\nList[0]{@1}").unwrap();

    let converted = boa(&["convert", text_path.to_str().unwrap()]);
    assert!(converted.status.success(), "{:?}", converted);
    assert!(binary_path.is_file());

    for algorithm in ["naive", "nlogn"] {
        let output = boa(&[algorithm, binary_path.to_str().unwrap()]);
        assert!(output.status.success(), "{:?}", output);
        let stdout = String::from_utf8(output.stdout).unwrap();
        assert!(
            stdout.contains(&format!("algorithm: {algorithm}")),
            "{stdout}"
        );
        assert!(stdout.contains("n_states: 2"), "{stdout}");
        assert!(stdout.contains("n_states_min: 1"), "{stdout}");
    }
}

#[test]
fn cli_reports_help_and_rejects_missing_arguments() {
    let help = boa(&["--help"]);
    assert!(help.status.success(), "{:?}", help);
    assert!(String::from_utf8(help.stdout).unwrap().contains("USAGE:"));

    let missing = boa(&[]);
    assert!(!missing.status.success());
    assert!(String::from_utf8(missing.stderr)
        .unwrap()
        .contains("USAGE:"));
}
