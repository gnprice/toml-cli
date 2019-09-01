use std::fs;
use std::path::PathBuf;
use std::str;

use failure::{Error, Fail};
use regex::Regex;
use serde::ser::{Serialize, SerializeMap, Serializer, SerializeSeq};
use structopt::StructOpt;
use toml_edit::{Document, Item};

#[derive(StructOpt)]
enum Args {
    Get {
        #[structopt(parse(from_os_str))]
        path: PathBuf,
        query: String,
    },
    // TODO: set
    // TODO: append/add (name TBD)
}

#[derive(Debug, Fail)]
enum CliError {
    #[fail(display = "bad query")]
    BadQuery(),
}

fn main() -> Result<(), Error> {
    let args = Args::from_args();
    match args {
        Args::Get { path, query } => get(path, &query)?,
    }
    Ok(())
}

fn get(path: PathBuf, query: &str) -> Result<(), Error> {
    let query = parse_query(query)?;

    // TODO: better report errors like ENOENT
    let data = fs::read(path)?;
    let data = str::from_utf8(&data)?;
    let doc = data.parse::<Document>()?;

    let item = walk_query(&doc.root, &query);

    // TODO: support shell-friendly output like `jq -r`
    println!("{}", serde_json::to_string(&JsonItem(item))?);

    /*
    doc["package"]["foo"] = value("bar");
    println!("{}", doc.to_string());
    */
    Ok(())
}

struct Query<'a>(Vec<QueryComponent<'a>>);

enum QueryComponent<'a> {
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
            r.0.push(QueryComponent::Name(cap.get(1).unwrap().as_str()));
            query = &query[cap.get(0).unwrap().end()..];
        } else if let Some(cap) = re_bynum.captures(&query) {
            r.0.push(QueryComponent::Num(
                cap.get(1).unwrap().as_str().parse::<usize>().unwrap()));
            query = &query[cap.get(0).unwrap().end()..];
        } else if query == "" {
            break;
        } else {
            Err(CliError::BadQuery())?; // TODO: better message
        }
    }

    Ok(r)
}

fn walk_query<'a>(mut item: &'a toml_edit::Item, query: &Query)
              -> &'a toml_edit::Item {
    for qc in &query.0 {
        match qc {
            QueryComponent::Name(n) => item = &item[n],
            QueryComponent::Num(n) => item = &item[n],
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
