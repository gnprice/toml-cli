use std::env;
use std::ffi::OsString;
use std::path::PathBuf;
use std::process;
use std::str;

#[test]
fn help_if_no_args() {
    // Probably want to factor out much of this when adding more tests.
    let proc = process::Command::new(get_exec_path()).output().unwrap();
    assert!(!proc.status.success());
    let stderr = str::from_utf8(proc.stderr.as_slice()).unwrap();
    assert!(stderr.contains("-h, --help"));
}

fn get_exec_path() -> PathBuf {
    // TODO is there no cleaner way to get this from Cargo?
    // Also should it really be "debug"?
    let target_dir: PathBuf = env::var_os("CARGO_TARGET_DIR")
        .unwrap_or_else(|| OsString::from("target"))
        .into();
    target_dir.join("debug").join("toml")
}
