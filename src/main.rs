use std::fs;
use std::path::PathBuf;
use std::str;

use failure::{Error, Fail};
use regex::Regex;
use serde::ser::{Serialize, SerializeMap, Serializer, SerializeSeq};
use structopt::StructOpt;
use toml_edit::{Document, Item, Table, Value, value};

#[derive(StructOpt)]
enum Args {
    Get {
        #[structopt(parse(from_os_str))]
        path: PathBuf,
        query: String,
        #[structopt(flatten)]
        opts: GetOpts,
    },
    Set {
        #[structopt(parse(from_os_str))]
        path: PathBuf,
        query: String,
        value_str: String, // TODO more forms
    },
    // TODO: append/add (name TBD)
}

#[derive(StructOpt)]
struct GetOpts {
    #[structopt(long)]
    output_toml: bool,
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
    let tpath = parse_query(query)?.0;
    let doc = read_parse(path)?;

    if opts.output_toml {
        print_toml_fragment(&doc, &tpath);
    } else {
        let item = walk_tpath(&doc.root, &tpath);
        // TODO: support shell-friendly output like `jq -r`
        println!("{}", serde_json::to_string(&JsonItem(item))?);
    }
    Ok(())
}

fn print_toml_fragment(doc: &Document, tpath: &[TpathSegment]) -> () {
    let item = walk_tpath(&doc.root, tpath);

    // TODO really need to use tpath -- makes no sense to print without that
    let mut doc = Document::new();
    match item {
        Item::Table(_) => doc.root = item.clone(),
        Item::ArrayOfTables(_) => {
            let mut t = Table::new();
            t[""] = item.clone();
            doc.root = Item::Table(t);
        }
        Item::Value(_) => {
            let mut t = Table::new();
            t[""] = item.clone();
            doc.root = Item::Table(t);
        }
        Item::None => (),
    }
    println!("{}", doc.to_string());
}

fn set(path: PathBuf, query: &str, value_str: &str) -> Result<(), Error> {
    let tpath = parse_query(query)?.0;
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
    println!("{}", doc.to_string());
    Ok(())
}

/// Query language is simple: a query is a "TOML path", or tpath.
struct Query<'a>(Vec<TpathSegment<'a>>);

enum TpathSegment<'a> {
    Name(&'a str),
    Num(usize),
}

fn parse_query(mut query: &str) -> Result<Query, CliError> {
    let mut r = Query(vec![]);

    if query == "." {
        return Ok(r);
    }

    let re_byname = Regex::new(r"\A\.(\w+)").unwrap();
    let re_bynum = Regex::new(r"\A\[(\d+)\]").unwrap();
    loop {
        if let Some(cap) = re_byname.captures(&query) {
            r.0.push(TpathSegment::Name(cap.get(1).unwrap().as_str()));
            query = &query[cap.get(0).unwrap().end()..];
        } else if let Some(cap) = re_bynum.captures(&query) {
            let n = match cap.get(1).unwrap().as_str().parse::<usize>() {
                Err(_) => Err(CliError::BadQuery())?, // TODO: specific message
                Ok(n) => n,
            };
            r.0.push(TpathSegment::Num(n));
            query = &query[cap.get(0).unwrap().end()..];
        } else if query == "" {
            break;
        } else {
            Err(CliError::BadQuery())?; // TODO: better message
        }
    }

    Ok(r)
}

fn walk_tpath<'a>(mut item: &'a toml_edit::Item, tpath: &[TpathSegment])
              -> &'a toml_edit::Item {
    for seg in tpath {
        match seg {
            TpathSegment::Name(n) => item = &item[n],
            TpathSegment::Num(n) => item = &item[n],
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
