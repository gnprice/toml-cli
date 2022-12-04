use std::env;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::PathBuf;
use std::process;
use std::process::Output;
use std::str;

use tempfile::TempDir;

#[test]
fn help_if_no_args() {
    let err = toml_error([] as [&str; 0]);
    assert!(err.contains("-h, --help"));
}

#[test]
fn cmd_get() {
    let contents = r#"[a]
b = "c"
[x]
y = "z""#;
    let (toml_file, _tempdir) = prep_file(contents);

    let actual = toml_success(["get", &toml_file, "x.y"]);
    assert_eq!("\"z\"\n", actual);

    let actual = toml_success(["get", "--raw", &toml_file, "x.y"]);
    assert_eq!("z\n", actual);

    // x.z does not exist
    toml_error(["get", &toml_file, "x.z"]);
}

#[test]
fn cmd_set() {
    let contents = r#"[a]
b = "c"
[x]
y = "z""#;
    let (toml_file, _tempdir) = prep_file(contents);

    // x.y exists
    let actual = toml_success(["set", &toml_file, "x.y", "new"]);
    let expected = r#"[a]
b = "c"
[x]
y = "new"
"#;
    assert_eq!(expected, actual);

    let actual = toml_success(["set", &toml_file, "x.z", "123"]);
    let expected = r#"[a]
b = "c"
[x]
y = "z"
z = "123"
"#;
    assert_eq!(expected, actual);
}

fn toml_success<I, S>(args: I) -> String
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let out = run_toml(args);
    assert!(out.status.success());
    assert!(out.stderr.is_empty());
    String::from_utf8(out.stdout).unwrap()
}

fn toml_error<I, S>(args: I) -> String
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let out = run_toml(args);
    assert!(!out.status.success());
    assert!(out.stdout.is_empty());
    String::from_utf8(out.stderr).unwrap()
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

fn prep_file(contents: &str) -> (String, TempDir) {
    let toml_dir = tempfile::tempdir().expect("failed to create tempdir");
    let toml_file = toml_dir.path().join("test.toml");
    fs::write(&toml_file, contents).expect("failed to write tempfile");
    (
        String::from(toml_file.as_os_str().to_str().unwrap()),
        toml_dir,
    )
}
