use std::fs;
use std::process;
use std::str;

const TOML_CMD: &str = "toml";

#[test]
fn integration_test_help_if_no_args() {
    // Probably want to factor out much of this when adding more tests.
    let cmd = process::Command::new(TOML_CMD).output().unwrap();
    assert!(!cmd.status.success());
    let stderr = str::from_utf8(cmd.stderr.as_slice()).unwrap();
    assert!(stderr.contains("-h, --help"));
}

#[test]
fn integration_test_cmd_get() {
    let body = r#"[a]
b = "c"
[x]
y = "z""#;
    let toml_dir = tempfile::tempdir().expect("failed to create tempdir");
    let toml_file = toml_dir.path().join("test.toml");
    fs::write(&toml_file, body).expect("failed to write tempfile");
    let toml_file = toml_file.as_os_str().to_str().unwrap();

    let cmd = process::Command::new(TOML_CMD)
        .args(["get", toml_file, "x.y"])
        .output()
        .unwrap();
    assert!(cmd.status.success());
    let stdout = str::from_utf8(cmd.stdout.as_slice()).unwrap();
    assert_eq!("\"z\"\n", stdout);

    // x.z does not exists
    let cmd = process::Command::new(TOML_CMD)
        .args(["get", toml_file, "x.z"])
        .output()
        .unwrap();
    assert!(!cmd.status.success());
}

#[test]
fn integration_test_cmd_set() {
    // fn set(path: PathBuf, query: &str, value_str: &str, opts: SetOpts) -> Result<(), Error> {
    let body = r#"[a]
b = "c"
[x]
y = "z""#;
    let toml_dir = tempfile::tempdir().expect("failed to create tempdir");
    let toml_file = toml_dir.path().join("test.toml");
    fs::write(&toml_file, body).expect("failed to write tempfile");
    let toml_file = toml_file.as_os_str().to_str().unwrap();

    // x.y exists
    let cmd = process::Command::new(TOML_CMD)
        .args(["set", toml_file, "x.y", "new"])
        .output()
        .unwrap();
    assert!(cmd.status.success());
    let stdout = str::from_utf8(cmd.stdout.as_slice()).unwrap();
    let excepted = r#"[a]
b = "c"
[x]
y = "new"
"#;
    assert_eq!(excepted, stdout);

    let cmd = process::Command::new(TOML_CMD)
        .args(["set", toml_file, "x.z", "123"])
        .output()
        .unwrap();
    assert!(cmd.status.success());
    let stdout = str::from_utf8(cmd.stdout.as_slice()).unwrap();
    let excepted = r#"[a]
b = "c"
[x]
y = "z"
z = "123"
"#;
    assert_eq!(excepted, stdout);
}
