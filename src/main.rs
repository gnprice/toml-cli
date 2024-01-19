mod query_parser;

use std::path::PathBuf;
use std::str;
use std::{fs, process::exit};

use anyhow::Error;
use serde::ser::{Serialize, SerializeMap, SerializeSeq, Serializer};
use structopt::StructOpt;
use thiserror::Error;
use toml_edit::{value, Document, Item, Table, Value};

use query_parser::{parse_query, Query, TpathSegment};

// TODO: Get more of the description in the README into the CLI help.
#[derive(StructOpt)]
#[structopt(about)]
enum Args {
    /// Print some data from the file
    ///
    /// Read the given TOML file, find the data within it at the given query,
    /// and print.
    ///
    /// If the TOML document does not have the given key, exit with a
    /// failure status.
    ///
    /// Output is JSON by default.  With `--raw`/`-r`, if the data is a
    /// string, print it directly.  With `--output-toml`, print the data
    /// as a fragment of TOML.
    // Without verbatim_doc_comment, the paragraphs get rewrapped to like
    // 120 columns wide.
    #[structopt(verbatim_doc_comment)]
    Get {
        /// Path to the TOML file to read
        #[structopt(parse(from_os_str))]
        path: PathBuf,

        /// Query within the TOML data (e.g. `dependencies.serde`, `foo[0].bar`)
        query: String,

        #[structopt(flatten)]
        opts: GetOpts,
    },

    /// Edit the file to set some data.
    ///
    /// Specify `--write`/`-w` to write back to the input file, otherwise the
    /// modified file is printed.
    Set {
        /// Path to the TOML file to read
        #[structopt(parse(from_os_str))]
        path: PathBuf,

        /// Query within the TOML data (e.g. `dependencies.serde`, `foo[0].bar`)
        query: String,

        /// String value to place at the given spot (bool, array, etc. are TODO)
        value_str: String, // TODO more forms

        /// Write back to file
        #[structopt(short, long)]
        write: bool,
    },
    //
    // TODO: append/add (name TBD)
}

#[derive(StructOpt)]
struct GetOpts {
    /// Print as a TOML fragment (default: print as JSON)
    #[structopt(long)]
    output_toml: bool,

    /// Print strings raw, not as JSON
    // (No effect when the item isn't a string, just like `jq -r`.)
    #[structopt(long, short)]
    raw: bool,
}

#[derive(Debug, Error)]
enum CliError {
    #[error("syntax error in query: {0}")]
    QuerySyntaxError(String),
    #[error("numeric index into non-array")]
    NotArray(),
    #[error("array index out of bounds")]
    ArrayIndexOob(),
}

/// An error that should cause a failure exit, but no message on stderr.
#[derive(Debug, Error)]
enum SilentError {
    #[error("key not found: {key}")]
    KeyNotFound { key: String },
}

fn main() {
    let args = Args::from_args();
    let result = match args {
        Args::Get { path, query, opts } => get(&path, &query, &opts),
        Args::Set {
            path,
            query,
            value_str,
            write,
        } => set(&path, &query, &value_str, write),
    };
    result.unwrap_or_else(|err| {
        match err.downcast::<SilentError>() {
            Ok(_) => {}
            Err(err) => {
                eprintln!("toml: {}", err);
            }
        }
        exit(1);
    })
}

fn read_parse(path: &PathBuf) -> Result<Document, Error> {
    // TODO: better report errors like ENOENT
    let data = fs::read(path)?;
    let data = str::from_utf8(&data)?;
    Ok(data.parse::<Document>()?)
}

fn get(path: &PathBuf, query: &str, opts: &GetOpts) -> Result<(), Error> {
    let tpath = parse_query_cli(query)?.0;
    let doc = read_parse(path)?;

    if opts.output_toml {
        print_toml_fragment(&doc, &tpath);
        return Ok(());
    }

    let item = walk_tpath(doc.as_item(), &tpath);
    let item = item.ok_or(SilentError::KeyNotFound { key: query.into() })?;

    if opts.raw {
        if let Item::Value(Value::String(s)) = item {
            println!("{}", s.value());
            return Ok(());
        }
    }

    println!("{}", serde_json::to_string(&JsonItem(item))?);
    Ok(())
}

fn print_toml_fragment(doc: &Document, tpath: &[TpathSegment]) {
    use TpathSegment::{Name, Num};

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
    print!("{}", doc);
}

fn set(path: &PathBuf, query: &str, value_str: &str, write: bool) -> Result<(), Error> {
    let tpath = parse_query_cli(query)?.0;
    let mut doc = read_parse(path)?;

    let mut item = doc.as_item_mut();
    let mut already_inline = false;
    let mut tpath = &tpath[..];
    use TpathSegment::{Name, Num};
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
    *item = value(value_str);

    if write {
        fs::write(path, doc.to_string())?;
    } else {
        print!("{}", doc);
    }

    Ok(())
}

fn parse_query_cli(query: &str) -> Result<Query, CliError> {
    parse_query(query).map_err(|_err| {
        CliError::QuerySyntaxError(query.into()) // TODO: perhaps use parse-error details?
    })
}

fn walk_tpath<'a>(
    mut item: &'a toml_edit::Item,
    tpath: &[TpathSegment],
) -> Option<&'a toml_edit::Item> {
    use TpathSegment::{Name, Num};
    for seg in tpath {
        match seg {
            Name(n) => item = item.get(n)?,
            Num(n) => item = item.get(n)?,
        }
    }
    Some(item)
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
