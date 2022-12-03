use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::PathBuf;
use std::process;
use std::str;

#[test]
fn integration_test_help_if_no_args() {
    // Probably want to factor out much of this when adding more tests.
    let proc = process::Command::new(get_exec_path()).output().unwrap();
    assert!(!proc.status.success());
    let stderr = str::from_utf8(proc.stderr.as_slice()).unwrap();
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

    let cmd = process::Command::new(get_exec_path())
        .args(["get", toml_file, "x.y"])
        .output()
        .unwrap();
    assert!(cmd.status.success());
    let stdout = str::from_utf8(cmd.stdout.as_slice()).unwrap();
    assert_eq!("\"z\"\n", stdout);

    // x.z does not exists
    let cmd = process::Command::new(get_exec_path())
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
    let cmd = process::Command::new(get_exec_path())
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

    let cmd = process::Command::new(get_exec_path())
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

fn get_exec_path() -> PathBuf {
    // TODO is there no cleaner way to get this from Cargo?
    // Also should it really be "debug"?
    let target_dir: PathBuf = env::var_os("CARGO_TARGET_DIR")
        .unwrap_or_else(|| OsString::from("target"))
        .into();
    target_dir.join("debug").join("toml")
}
