use std::env;
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

macro_rules! tomltest_get_err {
    ($name:ident, $args:expr, $pattern:expr) => {
        tomltest!($name, |mut t: TestCaseState| {
            t.write_file(INPUT);
            t.cmd.args(["get", &t.filename()]).args($args);
            check_contains($pattern, &t.expect_error());
        });
    };
}

macro_rules! tomltest_get_err_empty {
    ($name:ident, $args:expr) => {
        tomltest!($name, |mut t: TestCaseState| {
            t.write_file(INPUT);
            t.cmd.args(["get", &t.filename()]).args($args);
            check_eq("", &t.expect_error());
        });
    };
}

macro_rules! tomltest_get {
    ($name:ident, $args:expr, $expected:expr) => {
        tomltest!($name, |mut t: TestCaseState| {
            t.write_file(INPUT);
            t.cmd.args(["get", &t.filename()]).args($args);
            check_eq($expected, &t.expect_success());
        });
    };
}

macro_rules! tomltest_get1 {
    ($name:ident, $key:expr, $expected:expr) => {
        tomltest!($name, |mut t: TestCaseState| {
            t.write_file(INPUT);
            t.cmd.args(["get", &t.filename(), $key]);
            let expected = format!("{}\n", serde_json::to_string(&$expected).unwrap());
            check_eq(&expected, &t.expect_success());
        });
    };
}

tomltest!(help_if_no_args, |mut t: TestCaseState| {
    check_contains("-h, --help", &t.expect_error());
});

const INPUT: &str = r#"
key = "value"
int = 17
bool = true

# this is a TOML comment
bare-Key_1 = "bare"  # another TOML comment
"quoted key‽" = "quoted"
"" = "empty"
dotted.a = "dotted-a"
dotted . b = "dotted-b"

[foo]
x = "foo-x"
y.yy = "foo-yy"
"#;

tomltest_get1!(get_string, "key", "value");
tomltest_get1!(get_int, "int", 17);
tomltest_get1!(get_bool, "bool", true);
// TODO test remaining TOML value types: float, datetime, and aggregates:
//   array, table, inline table, array of tables.

// Test the various TOML key syntax: https://toml.io/en/v1.0.0#keys
tomltest_get1!(get_bare_key, "bare-Key_1", "bare");
tomltest_get1!(get_quoted_key, "\"quoted key‽\"", "quoted");
tomltest_get1!(get_empty_key, "\"\"", "empty");
tomltest_get1!(get_dotted_key, "dotted.a", "dotted-a");
tomltest_get1!(get_dotted_spaced_key, "dotted.b", "dotted-b");
tomltest_get1!(get_nested, "foo.x", "foo-x");
tomltest_get1!(get_nested_dotted, "foo.y.yy", "foo-yy");
// TODO test `get` inside arrays and arrays of tables

tomltest_get!(get_string_raw, ["--raw", "key"], "value\n");
// TODO test `get --raw` on non-strings

// TODO test `get --output-toml`

tomltest_get_err!(get_invalid_query, [".bad"], "syntax error in query: .bad");
tomltest_get_err_empty!(get_missing, ["nosuchkey"]);
tomltest_get_err_empty!(get_missing_num, ["key[1]"]);

macro_rules! tomltest_set {
    ($name:ident, $args:expr, $expected:literal) => {
        tomltest!($name, |mut t: TestCaseState| {
            t.write_file(&("\n".to_owned() + INITIAL + "\n"));
            t.cmd.args(["set", &t.filename()]).args($args);
            check_eq(&format!($expected), &t.expect_success());
        });
    };
}

const INITIAL: &str = "[x]\ny = 1";

tomltest_set!(
    set_auto_string_replace_existing,
    ["x.y", "update"],
    r#"
[x]
y = "update"
"#
);

tomltest_set!(
    set_auto_string_extend_existing,
    ["x.z", "new"],
    r#"
{INITIAL}
z = "new"
"#
);

tomltest_set!(
    set_auto_string_new_table,
    ["foo.bar", "baz"],
    r#"
{INITIAL}

[foo]
bar = "baz"
"#
);

tomltest_set!(
    set_auto_string_new_toplevel,
    ["foo", "bar"],
    r#"foo = "bar"

{INITIAL}
"#
);

tomltest_set!(
    set_auto_bool_replace_existing,
    ["x.y", "true"],
    r#"
[x]
y = true
"#
);

tomltest_set!(
    set_auto_string_array_replace_existing,
    ["x.y", r#"["a", "b"]"#],
    r#"
[x]
y = ["a", "b"]
"#
);

tomltest_set!(
    set_require_string_for_intlike_value,
    ["x.y", "2", "--type", "string"],
    r#"
[x]
y = "2"
"#
);

tomltest_set!(
    set_auto_string_with_quoted_string,
    ["x.y", r#""update""#],
    r#"
[x]
y = "\"update\""
"#
);

macro_rules! tomltest_set_err {
    ($name:ident, $args:expr, $pattern:expr) => {
        tomltest!($name, |mut t: TestCaseState| {
            t.write_file(&("\n".to_owned() + INITIAL + "\n"));
            t.cmd.args(["set", &t.filename()]).args($args);
            check_contains($pattern, &t.expect_error());
        });
    };
}

tomltest_set_err!(
    set_require_int_got_string,
    ["x.y", "foo", "--type", "int"],
    "toml: value type does not match required type"
);

tomltest_set_err!(
    set_require_bool_got_int,
    ["x.y", "2", "--type", "boolean"],
    "toml: value type does not match required type"
);

// TODO test `set` on string with newlines and other fun characters
// TODO test `set` when existing value is an array, table, or array of tables
// TODO test `set` inside existing array or inline table
// TODO test `set` inside existing array of tables

struct TestCaseState {
    cmd: process::Command,
    #[allow(dead_code)] // We keep the TempDir around to prolong its lifetime
    dir: TempDir,
    filename: PathBuf,
}

impl TestCaseState {
    pub fn new() -> Self {
        let cmd = process::Command::new(env!("CARGO_BIN_EXE_toml"));
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

/// Like `assert!(actual.contains(pattern))`, but with more informative output.
#[rustfmt::skip]
fn check_contains(pattern: &str, actual: &str) {
    if actual.contains(pattern) {
        return;
    }
    panic!("
/~~ expected pattern:
{}
/~~ got:
{}/~~
", pattern, actual);
}

/// Like `assert_eq!`, but with more-readable output for debugging failed tests.
///
/// In particular, print the strings directly rather than with `{:?}`.
#[rustfmt::skip]
fn check_eq(expected: &str, actual: &str) {
    if expected != actual {
        panic!("
~~~ expected:
{}~~~ got:
{}~~~
", expected, actual);
    }
}
