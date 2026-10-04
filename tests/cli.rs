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

#[test]
fn cli_refines_unit_add_inputs_and_weighted_fallback() {
    let temp_dir = TempDir::new();
    let cases = [
        ("flat", "Add[0]{@1:1}\nAdd[0]{@0:1,@2:1}\nAdd[0]{@1:1}", 2, 4),
        ("nested", "Add[0]{Add[0]{@0:1,@1:1}:1,Add[0]{@0:1}:1}\nAdd[0]{Add[0]{@0:1,@1:1}:1,Add[0]{@1:1,@2:1}:1}\nAdd[0]{Add[0]{@1:1,@2:1}:1}", 3, 9),
        ("weighted", "Add[0]{@0:2}\nAdd[0]{@1:2}\nAdd[0]{@2:3}", 2, 3),
    ];
    for (name, input, classes, edges) in cases {
        let text_path = temp_dir.0.join(format!("{name}.boa.txt"));
        let binary_path = temp_dir.0.join(format!("{name}.boa"));
        fs::write(&text_path, input).unwrap();
        assert!(boa(&["convert", text_path.to_str().unwrap()])
            .status
            .success());
        for algorithm in ["naive", "nlogn"] {
            let output = boa(&[algorithm, binary_path.to_str().unwrap()]);
            assert!(output.status.success(), "{name}: {output:?}");
            let stdout = String::from_utf8(output.stdout).unwrap();
            assert!(stdout.contains("n_states: 3\n"), "{stdout}");
            assert!(
                stdout.contains(&format!("n_states_min: {classes}\n")),
                "{stdout}"
            );
            if algorithm == "nlogn" {
                assert!(stdout.contains(&format!("m_edges: {edges}\n")), "{stdout}");
                for field in [
                    "parsing_time_s",
                    "backrefs_time_s",
                    "iter_time_s",
                    "selfreport_time_s",
                ] {
                    assert!(stdout.contains(&format!("{field}: ")), "{stdout}");
                }
            }
        }
    }
}

#[test]
fn cli_refines_empty_inputs() {
    let temp_dir = TempDir::new();
    let text_path = temp_dir.0.join("empty.boa.txt");
    let binary_path = temp_dir.0.join("empty.boa");
    fs::write(&text_path, "").unwrap();
    assert!(boa(&["convert", text_path.to_str().unwrap()])
        .status
        .success());
    for algorithm in ["naive", "nlogn"] {
        let output = boa(&[algorithm, binary_path.to_str().unwrap()]);
        assert!(output.status.success(), "{output:?}");
        let stdout = String::from_utf8(output.stdout).unwrap();
        assert!(stdout.contains("n_states: 0\n"), "{stdout}");
        assert!(stdout.contains("n_states_min: 0\n"), "{stdout}");
    }
}
