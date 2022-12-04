use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::PathBuf;
use std::process;
use std::process::Output;
use std::str;

use tempfile::TempDir;

macro_rules! tomltest {
    ($name:ident, $fun:expr) => {
        #[test]
        fn $name() {
            $fun(TestCaseState::new());
        }
    };
}

tomltest!(help_if_no_args, |mut t: TestCaseState| {
    assert!(t.expect_error().contains("-h, --help"));
});

tomltest!(get_string, |mut t: TestCaseState| {
    let contents = r#"[a]
b = "c"
[x]
y = "z""#;
    t.write_file(contents);
    t.cmd.args(["get", &t.filename(), "x.y"]);
    assert_eq!("\"z\"\n", t.expect_success());
});

tomltest!(get_string_raw, |mut t: TestCaseState| {
    let contents = r#"[a]
b = "c"
[x]
y = "z""#;
    t.write_file(contents);
    t.cmd.args(["get", "--raw", &t.filename(), "x.y"]);
    assert_eq!("z\n", t.expect_success());
});

tomltest!(get_missing, |mut t: TestCaseState| {
    let contents = r#"[a]
b = "c"
[x]
y = "z""#;
    t.write_file(contents);
    t.cmd.args(["get", &t.filename(), "x.z"]);
    t.expect_error();
});

tomltest!(set_string_existing, |mut t: TestCaseState| {
    let contents = r#"[a]
b = "c"
[x]
y = "z""#;
    t.write_file(contents);
    t.cmd.args(["set", &t.filename(), "x.y", "new"]);
    let expected = r#"[a]
b = "c"
[x]
y = "new"
"#;
    assert_eq!(expected, t.expect_success());
});

tomltest!(set_string, |mut t: TestCaseState| {
    let contents = r#"[a]
b = "c"
[x]
y = "z""#;
    t.write_file(contents);
    t.cmd.args(["set", &t.filename(), "x.z", "123"]);
    let expected = r#"[a]
b = "c"
[x]
y = "z"
z = "123"
"#;
    assert_eq!(expected, t.expect_success());
});

struct TestCaseState {
    cmd: process::Command,
    #[allow(dead_code)] // We keep the TempDir around to prolong its lifetime
    dir: TempDir,
    filename: PathBuf,
}

impl TestCaseState {
    pub fn new() -> Self {
        let cmd = process::Command::new(get_exec_path());
        let dir = tempfile::tempdir().expect("failed to create tempdir");
        let filename = dir.path().join("test.toml");
        TestCaseState { cmd, dir, filename }
    }

    pub fn expect_success(&mut self) -> String {
        let out = self.cmd.output().unwrap();
        if !out.status.success() {
            self.fail(&out, "Command failed!");
        } else if !out.stderr.is_empty() {
            self.fail(&out, "Command printed to stderr despite success");
        }
        String::from_utf8(out.stdout).unwrap()
    }

    pub fn expect_error(&mut self) -> String {
        let out = self.cmd.output().unwrap();
        if out.status.success() {
            self.fail(&out, "Command succeeded; expected failure");
        } else if !out.stdout.is_empty() {
            self.fail(&out, "Command printed to stdout despite failure");
        }
        String::from_utf8(out.stderr).unwrap()
    }

    fn fail(&self, out: &Output, summary: &str) {
        panic!(
            "\n============\
             \n{}\
             \ncmdline: {:?}\
             \nstatus: {}\
             \nstderr: {}\
             \nstdout: {}\
             \n============\n",
            summary,
            self.cmd,
            out.status,
            String::from_utf8_lossy(&out.stderr),
            String::from_utf8_lossy(&out.stdout),
        )
    }

    pub fn write_file(&self, contents: &str) {
        fs::write(&self.filename, contents).expect("failed to write test fixture");
    }

    pub fn filename(&self) -> String {
        // TODO we don't really need a String here, do we?
        String::from(self.filename.as_os_str().to_str().unwrap())
    }
}

fn get_exec_path() -> PathBuf {
    // TODO is there no cleaner way to get this from Cargo?
    // Also should it really be "debug"?
    let target_dir: PathBuf = env::var_os("CARGO_TARGET_DIR")
        .unwrap_or_else(|| OsString::from("target"))
        .into();
    target_dir.join("debug").join("toml")
}
