mod query_parser;

use std::fs;
use std::path::PathBuf;
use std::str;

use failure::{Error, Fail};
use serde::ser::{Serialize, SerializeMap, Serializer, SerializeSeq};
use structopt::StructOpt;
use toml_edit::{Document, Item, Table, Value, value};

use query_parser::{Query, TpathSegment, parse_query};

// TODO: Get more of the description in the README into the CLI help.
#[derive(StructOpt)]
#[structopt(about)]
enum Args {
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
    /// Edit the file to set some data (currently, just print modified version)
    Set {
        /// Path to the TOML file to read
        #[structopt(parse(from_os_str))]
        path: PathBuf,
        /// Query within the TOML data (e.g. `dependencies.serde`, `foo[0].bar`)
        query: String,
        /// String value to place at the given spot (bool, array, etc. are TODO)
        value_str: String, // TODO more forms
    },
    // TODO: append/add (name TBD)
}

#[derive(StructOpt)]
struct GetOpts {
    /// Print as a TOML fragment (default: print as JSON)
    #[structopt(long)]
    output_toml: bool,
    #[structopt(long, short)]
    raw: bool,
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
        Args::Get { path, query, opts } => get(path, &query, opts)?,
        Args::Set { path, query, value_str } => set(path, &query, &value_str)?,
    }
    Ok(())
}

fn read_parse(path: PathBuf) -> Result<Document, Error> {
    // TODO: better report errors like ENOENT
    let data = fs::read(path)?;
    let data = str::from_utf8(&data)?;
    Ok(data.parse::<Document>()?)
}

fn get(path: PathBuf, query: &str, opts: GetOpts) -> Result<(), Error> {
    let tpath = parse_query_cli(query)?.0;
    let doc = read_parse(path)?;

    if opts.output_toml {
        print_toml_fragment(&doc, &tpath);
    } else if opts.raw {
        let item = walk_tpath(&doc.root, &tpath);
        let item = serde_json::to_value(&JsonItem(item))?;
        match item {
            serde_json::Value::String(s) => println!("{}", s),
            _ => {
                println!("{}", serde_json::to_string(&item)?);
            }
        }
    } else {
        let item = walk_tpath(&doc.root, &tpath);
        println!("{}", serde_json::to_string(&JsonItem(item))?);
    }
    Ok(())
}

fn print_toml_fragment(doc: &Document, tpath: &[TpathSegment]) -> () {
    use TpathSegment::{Name, Num};

    let mut item = &doc.root;
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
                    Item::Table(t) => { next.append(t); },
                    _ => panic!("malformed TOML parse-tree"),
                }
                item = Item::ArrayOfTables(next);
            }
            _ => panic!("UNIMPLEMENTED: --output-toml inside inline data"), // TODO
        }
    }
    let mut doc = Document::new();
    doc.root = item;
    print!("{}", doc.to_string());
}

fn set(path: PathBuf, query: &str, value_str: &str) -> Result<(), Error> {
    let tpath = parse_query_cli(query)?.0;
    let mut doc = read_parse(path)?;

    let mut item = &mut doc.root;
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
                match &item { Item::Value(_) => already_inline = true, _ => () };
                item = &mut item[n];
            }
            Name(n) => {
                match &item {
                    Item::Table(_) => (),
                    Item::Value(Value::InlineTable(_)) => already_inline = true,
                    // TODO make this more directly construct the new, inner part?
                    _ => *item = if already_inline {
                        Item::Value(Value::InlineTable(Default::default()))
                    } else {
                        Item::Table(Table::new())
                    },
                };
                item = &mut item[n];
            }
        }
    }
    *item = value(value_str);

    // TODO actually write back
    print!("{}", doc.to_string());
    Ok(())
}

fn parse_query_cli(query: &str) -> Result<Query, CliError> {
    parse_query(query).map_err(|_err| {
        CliError::BadQuery() // TODO: specific message
    })
}

fn walk_tpath<'a>(mut item: &'a toml_edit::Item, tpath: &[TpathSegment])
              -> &'a toml_edit::Item {
    use TpathSegment::{Name, Num};
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
    where S: Serializer,
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
    where S: Serializer,
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
    where S: Serializer,
    {
        if let Some(v) = self.0.as_integer() {
            v.serialize(serializer)
        } else if let Some(v) = self.0.as_float() {
            v.serialize(serializer)
        } else if let Some(v) = self.0.as_bool() {
            v.serialize(serializer)
        } else if let Some(v) = self.0.as_str() {
            v.serialize(serializer)
        } else if let Some(_) = self.0.as_date_time() {
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
