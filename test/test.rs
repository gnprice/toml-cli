use std::env;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::PathBuf;
use std::process;
use std::process::Output;
use std::str;

#[test]
fn integration_test_help_if_no_args() {
    // Probably want to factor out much of this when adding more tests.
    let proc = run_toml([] as [&str; 0]);
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

    let cmd = run_toml(["get", toml_file, "x.y"]);
    assert!(cmd.status.success());
    let stdout = str::from_utf8(cmd.stdout.as_slice()).unwrap();
    assert_eq!("\"z\"\n", stdout);

    // x.z does not exist
    let cmd = run_toml(["get", toml_file, "x.z"]);
    assert!(!cmd.status.success());
}

#[test]
fn integration_test_cmd_set() {
    let body = r#"[a]
b = "c"
[x]
y = "z""#;
    let toml_dir = tempfile::tempdir().expect("failed to create tempdir");
    let toml_file = toml_dir.path().join("test.toml");
    fs::write(&toml_file, body).expect("failed to write tempfile");
    let toml_file = toml_file.as_os_str().to_str().unwrap();

    // x.y exists
    let cmd = run_toml(["set", toml_file, "x.y", "new"]);
    assert!(cmd.status.success());
    let stdout = str::from_utf8(cmd.stdout.as_slice()).unwrap();
    let expected = r#"[a]
b = "c"
[x]
y = "new"
"#;
    assert_eq!(expected, stdout);

    let cmd = run_toml(["set", toml_file, "x.z", "123"]);
    assert!(cmd.status.success());
    let stdout = str::from_utf8(cmd.stdout.as_slice()).unwrap();
    let expected = r#"[a]
b = "c"
[x]
y = "z"
z = "123"
"#;
    assert_eq!(expected, stdout);
}

fn run_toml<I, S>(args: I) -> Output
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let mut cmd = process::Command::new(get_exec_path());
    cmd.args(args).output().unwrap()
}

fn get_exec_path() -> PathBuf {
    // TODO is there no cleaner way to get this from Cargo?
    // Also should it really be "debug"?
    let target_dir: PathBuf = env::var_os("CARGO_TARGET_DIR")
        .unwrap_or_else(|| OsString::from("target"))
        .into();
    target_dir.join("debug").join("toml")
}
