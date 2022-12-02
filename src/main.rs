mod query_parser;

use chrono::{DateTime, Utc};
use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::str;

use failure::{Error, Fail};
use serde::ser::{Serialize, SerializeMap, SerializeSeq, Serializer};
use structopt::StructOpt;
use toml_edit::{value, Document, Item, Table, Value};

use query_parser::{parse_query, Query, TpathSegment};
use TpathSegment::{Name, Num};

// TODO: Get more of the description in the README into the CLI help.
#[derive(StructOpt)]
#[structopt(about)]
enum Args {
    /// Check if a key exists
    Check {
        /// Path to the TOML file to read
        #[structopt(parse(from_os_str))]
        path: PathBuf,
        /// Query within the TOML data (e.g. `dependencies.serde`, `foo[0].bar`)
        query: String,
    },
    /// Print some data from the file
    Get {
        /// Path to the TOML file to read
        #[structopt(parse(from_os_str))]
        path: PathBuf,
        /// Query within the TOML data (e.g. `dependencies.serde`, `foo[0].bar`)
        query: String,
        #[structopt(flatten)]
        opts: GetOpts,
    },
    /// Edit the file to set some data
    Set {
        /// Path to the TOML file to read
        #[structopt(parse(from_os_str))]
        path: PathBuf,
        /// Query within the TOML data (e.g. `dependencies.serde`, `foo[0].bar`)
        query: String,
        /// String value to place at the given spot (bool, array, etc. are TODO)
        value_str: String, // TODO more forms
        #[structopt(flatten)]
        opts: SetOpts,
    },
    // TODO: append/add (name TBD)
}

#[derive(Clone, Copy, Default, StructOpt)]
struct GetOpts {
    /// Print as a TOML fragment (default: print as JSON)
    #[structopt(long)]
    output_toml: bool,
}

#[derive(Clone, Copy, Default, StructOpt)]
struct SetOpts {
    /// Overwrite the TOML file (default: print to stdout)
    #[structopt(long)]
    overwrite: bool,
    /// Create a backup file when `overwrite` is set(default: doesn't create a backup file)
    #[structopt(long)]
    backup: bool,
}

#[derive(Debug, Fail)]
enum CliError {
    #[fail(display = "bad query")]
    BadQuery(),
    #[fail(display = "numeric index into non-array")]
    NotArray(),
    #[fail(display = "array index out of bounds")]
    ArrayIndexOob(),
}

fn main() -> Result<(), Error> {
    let args = Args::from_args();
    match args {
        Args::Check { path, query } => check(path, &query),
        Args::Get { path, query, opts } => get(path, &query, opts)?,
        Args::Set {
            path,
            query,
            value_str,
            opts,
        } => set(path, &query, &value_str, opts)?,
    }
    Ok(())
}

fn read_parse(path: PathBuf) -> Result<Document, Error> {
    // TODO: better report errors like ENOENT
    let data = fs::read(path)?;
    let data = str::from_utf8(&data)?;
    Ok(data.parse::<Document>()?)
}

fn check_exists(path: PathBuf, query: &str) -> Result<bool, Error> {
    let tpath = parse_query_cli(query)?.0;
    let doc = read_parse(path)?;
    let mut item = doc.as_item();

    for seg in tpath {
        match seg {
            Name(n) => {
                let i = item.get(n);
                if i.is_none() {
                    return Ok(false);
                }
                item = i.unwrap();
            }
            Num(n) => item = &item[n],
        }
    }

    Ok(true)
}

/// Check whether a key exists.
/// It will print 'true' to stdout in case exists, and set exit code to '0'
/// otherwise it will print 'false' to stderr and set exit code to '1'
fn check(path: PathBuf, query: &str) {
    if let Ok(r) = check_exists(path, query) {
        if r {
            println!("true");
            std::process::exit(0);
        }
    }
    eprintln!("false");
    std::process::exit(1);
}

fn get(path: PathBuf, query: &str, opts: GetOpts) -> Result<(), Error> {
    let value = get_value(path, query, opts)?;
    if opts.output_toml {
        print!("{}", value);
    } else {
        println!("{}", value);
    }
    Ok(())
}

fn get_value(path: PathBuf, query: &str, opts: GetOpts) -> Result<String, Error> {
    let tpath = parse_query_cli(query)?.0;
    let doc = read_parse(path)?;

    let value = if opts.output_toml {
        format_toml_fragment(&doc, &tpath)
    } else {
        let item = walk_tpath(doc.as_item(), &tpath);
        // TODO: support shell-friendly output like `jq -r`
        serde_json::to_string(&JsonItem(item))?
    };

    Ok(value)
}

fn format_toml_fragment(doc: &Document, tpath: &[TpathSegment]) -> String {
    let mut item = doc.as_item();
    let mut breadcrumbs = vec![];
    for seg in tpath {
        breadcrumbs.push((item, seg));
        match seg {
            Name(n) => item = &item[n],
            Num(n) => item = &item[n],
        }
    }

    let mut item = item.clone();
    while let Some((parent, seg)) = breadcrumbs.pop() {
        match (seg, parent) {
            (Name(n), Item::Table(t)) => {
                // TODO clean up all this copying; may need more from toml_edit API
                let mut next = t.clone();
                while !next.is_empty() {
                    let (k, _) = next.iter().next().unwrap();
                    let k = String::from(k);
                    next.remove(&k);
                }
                next[n] = item;
                item = Item::Table(next);
            }
            (Num(_), Item::ArrayOfTables(a)) => {
                // TODO clean up this copying too
                let mut next = a.clone();
                next.clear();
                match item {
                    #[rustfmt::skip]
                    Item::Table(t) => { next.push(t); }
                    _ => panic!("malformed TOML parse-tree"),
                }
                item = Item::ArrayOfTables(next);
            }
            _ => panic!("UNIMPLEMENTED: --output-toml inside inline data"), // TODO
        }
    }
    let doc = Document::from(item.into_table().unwrap());
    format!("{}", doc)
}

fn set(path: PathBuf, query: &str, value_str: &str, opts: SetOpts) -> Result<(), Error> {
    let result = set_value(path, query, value_str, opts)?;
    if let Some(doc) = result {
        print!("{}", doc);
    }
    Ok(())
}

fn set_value(
    path: PathBuf,
    query: &str,
    value_str: &str,
    opts: SetOpts,
) -> Result<Option<String>, Error> {
    let tpath = parse_query_cli(query)?.0;
    let mut doc = read_parse(path.clone())?;

    let mut item = doc.as_item_mut();
    let mut already_inline = false;
    let mut tpath = &tpath[..];
    while let Some(seg) = tpath.first() {
        tpath = &tpath[1..]; // TODO simplify to `for`, unless end up needing a tail
        match seg {
            Num(n) => {
                let len = match &item {
                    Item::ArrayOfTables(a) => a.len(),
                    Item::Value(Value::Array(a)) => a.len(),
                    _ => Err(CliError::NotArray())?,
                };
                if n >= &len {
                    Err(CliError::ArrayIndexOob())?;
                }
                #[allow(clippy::single_match)]
                match &item {
                    Item::Value(_) => already_inline = true,
                    _ => (),
                };
                item = &mut item[n];
            }
            Name(n) => {
                match &item {
                    Item::Table(_) => (),
                    Item::Value(Value::InlineTable(_)) => already_inline = true,
                    // TODO make this more directly construct the new, inner part?
                    _ => {
                        *item = if already_inline {
                            Item::Value(Value::InlineTable(Default::default()))
                        } else {
                            Item::Table(Table::new())
                        }
                    }
                };
                item = &mut item[n];
            }
        }
    }
    *item = detect_value(value_str);

    let result = if opts.overwrite {
        // write content to path
        if opts.backup {
            let now: DateTime<Utc> = Utc::now();
            let ext = now.format("%Y%m%d-%H%M%S-%f");
            let backup_file = format!("{}.{}", path.display(), ext);
            fs::copy(path.clone(), backup_file)?;
        }
        let mut output = OpenOptions::new().write(true).truncate(true).open(path)?;
        write!(output, "{}", doc)?;
        None
    } else {
        Some(format!("{}", doc))
    };

    Ok(result)
}

fn detect_value(value_str: &str) -> Item {
    if let Ok(i) = value_str.parse::<i64>() {
        value(i)
    } else if let Ok(b) = value_str.parse::<bool>() {
        value(b)
    } else {
        value(value_str)
    }
}

fn parse_query_cli(query: &str) -> Result<Query, CliError> {
    parse_query(query).map_err(|_err| {
        CliError::BadQuery() // TODO: specific message
    })
}

fn walk_tpath<'a>(mut item: &'a toml_edit::Item, tpath: &[TpathSegment]) -> &'a toml_edit::Item {
    for seg in tpath {
        match seg {
            Name(n) => item = &item[n],
            Num(n) => item = &item[n],
        }
    }
    item
}

// TODO Can we do newtypes more cleanly than this?
struct JsonItem<'a>(&'a toml_edit::Item);

impl Serialize for JsonItem<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self.0 {
            Item::Value(v) => JsonValue(v).serialize(serializer),
            Item::Table(t) => JsonTable(t).serialize(serializer),
            Item::ArrayOfTables(a) => {
                let mut seq = serializer.serialize_seq(Some(a.len()))?;
                for t in a.iter() {
                    seq.serialize_element(&JsonTable(t))?;
                }
                seq.end()
            }
            Item::None => serializer.serialize_none(),
        }
    }
}

struct JsonTable<'a>(&'a toml_edit::Table);

impl Serialize for JsonTable<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(self.0.len()))?;
        for (k, v) in self.0.iter() {
            map.serialize_entry(k, &JsonItem(v))?;
        }
        map.end()
    }
}

struct JsonValue<'a>(&'a toml_edit::Value);

impl Serialize for JsonValue<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        #[allow(clippy::redundant_pattern_matching)]
        if let Some(v) = self.0.as_integer() {
            v.serialize(serializer)
        } else if let Some(v) = self.0.as_float() {
            v.serialize(serializer)
        } else if let Some(v) = self.0.as_bool() {
            v.serialize(serializer)
        } else if let Some(v) = self.0.as_str() {
            v.serialize(serializer)
        } else if let Some(_) = self.0.as_datetime() {
            "UNIMPLEMENTED: DateTime".serialize(serializer) // TODO
        } else if let Some(arr) = self.0.as_array() {
            let mut seq = serializer.serialize_seq(Some(arr.len()))?;
            for e in arr.iter() {
                seq.serialize_element(&JsonValue(e))?;
            }
            seq.end()
        } else if let Some(t) = self.0.as_inline_table() {
            let mut map = serializer.serialize_map(Some(t.len()))?;
            for (k, v) in t.iter() {
                map.serialize_entry(k, &JsonValue(v))?;
            }
            map.end()
        } else {
            panic!("unknown variant of toml_edit::Value");
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    // functions to test
    use super::check_exists;
    use super::detect_value;
    use super::{get_value, GetOpts};
    use super::{set_value, SetOpts};

    #[test]
    fn test_detect_value() {
        let i = detect_value("abc");
        assert_eq!("string", i.type_name());
        assert!(i.is_str());
        assert_eq!(Some("abc"), i.as_str());

        let i = detect_value("123");
        assert_eq!("integer", i.type_name());
        assert!(i.is_integer());
        assert_eq!(Some(123), i.as_integer());

        let i = detect_value("true");
        assert_eq!("boolean", i.type_name());
        assert!(i.is_bool());
        assert_eq!(Some(true), i.as_bool());
    }

    #[test]
    fn test_check_exists() {
        let body = r#"[a]
b = "c"
[x]
y = "z""#;
        let dir = tempfile::tempdir().expect("failed to create tempdir");
        let toml_file = dir.path().join("test.toml");
        fs::write(&toml_file, body).expect("failed to create tempfile");

        // x.y exists
        let result = check_exists(toml_file.clone(), "x.y");
        assert!(result.is_ok());
        assert!(result.unwrap());

        // x.z does not exists
        let result = check_exists(toml_file, "x.z");
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[test]
    fn test_get_value() {
        let body = r#"[a]
b = "c"
[x]
y = "z""#;
        let dir = tempfile::tempdir().expect("failed to create tempdir");
        let toml_file = dir.path().join("test.toml");
        fs::write(&toml_file, body).expect("failed to write tempfile");

        let opts = GetOpts::default();
        // x.y exists
        let result = get_value(toml_file.clone(), "x.y", opts);
        assert!(result.is_ok());
        assert_eq!("\"z\"", result.unwrap());

        // x.z does not exists
        // FIXME: get_value now will panic, it's not a well-desined API.
        let result = std::panic::catch_unwind(|| {
            let _ = get_value(toml_file.clone(), "x.z", opts);
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_set_value() {
        // fn set(path: PathBuf, query: &str, value_str: &str, opts: SetOpts) -> Result<(), Error> {
        let body = r#"[a]
b = "c"
[x]
y = "z""#;
        let dir = tempfile::tempdir().expect("failed to create tempdir");
        let toml_file = dir.path().join("test.toml");
        fs::write(&toml_file, body).expect("failed to write tempfile");

        let mut opts = SetOpts::default();
        // x.y exists
        let result = set_value(toml_file.clone(), "x.y", "new", opts);
        assert!(result.is_ok());
        let excepted = r#"[a]
b = "c"
[x]
y = "new"
"#;
        assert_eq!(excepted, result.unwrap().unwrap());

        let result = set_value(toml_file.clone(), "x.z", "123", opts);
        assert!(result.is_ok());
        let excepted = r#"[a]
b = "c"
[x]
y = "z"
z = 123
"#;
        assert_eq!(excepted, result.unwrap().unwrap());

        let result = set_value(toml_file.clone(), "x.z", "false", opts);
        assert!(result.is_ok());
        let excepted = r#"[a]
b = "c"
[x]
y = "z"
z = false
"#;
        assert_eq!(excepted, result.unwrap().unwrap());

        // test overwrite the original file
        opts.overwrite = true;
        let result = set_value(toml_file.clone(), "x.z", "false", opts);
        assert!(result.is_ok());
        println!("{:?}", result);
        // --overwrite will not generate any output.
        assert_eq!(None, result.unwrap());

        let excepted = r#"[a]
b = "c"
[x]
y = "z"
z = false
"#;
        let new_body = fs::read_to_string(toml_file).expect("failed to read TOML file");
        assert_eq!(excepted, new_body);
    }
}
