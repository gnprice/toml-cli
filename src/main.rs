use std::fs;
use std::path::PathBuf;
use std::str;

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
}

fn main() -> Result<(), Box<dyn std::error::Error + 'static>> {
    let args = Args::from_args();
    match args {
        Args::Get { path, query } => get(path, &query),
    }
}

fn get(path: PathBuf, mut query: &str) -> Result<(), Box<dyn std::error::Error>> {
    let data = fs::read(path)?;
    let data = str::from_utf8(&data)?;
    let doc = data.parse::<Document>()?;
    let mut item = &doc.root;

    let re_byname = Regex::new(r"\A\.(\w+)").unwrap();
    let re_bynum = Regex::new(r"\A\[(\d+)\]").unwrap();
    loop {
        if let Some(cap) = re_byname.captures(&query) {
            item = &item[cap.get(1).unwrap().as_str()];
            query = &query[cap.get(0).unwrap().end()..];
        } else if let Some(cap) = re_bynum.captures(&query) {
            item = &item[cap.get(1).unwrap().as_str().parse::<usize>().unwrap()];
            query = &query[cap.get(0).unwrap().end()..];
        } else {
            break;
        }
    }
//    println!("{:#?}", item);
    println!("{}", serde_json::to_string(&JsonItem{ inner: item })?);

    /*
    doc["package"]["foo"] = value("bar");
    println!("{}", doc.to_string());
    */
    Ok(())
}



struct JsonItem<'a> { inner: &'a toml_edit::Item }

impl Serialize for JsonItem<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self.inner {
            Item::Value(v) => JsonValue{ inner: v }.serialize(serializer),
            Item::Table(t) => {
                let mut map = serializer.serialize_map(Some(t.len()))?;
                for (k, v) in t.iter() {
                    map.serialize_entry(k, &JsonItem{ inner: v })?;
                }
                map.end()
            }
            _ => "UNIMPLEMENTED".serialize(serializer),
        }
    }
}

struct JsonValue<'a> { inner: &'a toml_edit::Value }

impl Serialize for JsonValue<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: Serializer,
    {
        if let Some(s) = self.inner.as_str() {
            s.serialize(serializer)
        } else if let Some(i) = self.inner.as_integer() {
            i.serialize(serializer)
        } else if let Some(arr) = self.inner.as_array() {
            let mut seq = serializer.serialize_seq(Some(arr.len()))?;
            for e in arr.iter() {
                seq.serialize_element(&JsonValue{ inner: e })?;
            }
            seq.end()
        } else if let Some(t) = self.inner.as_inline_table() {
            let mut map = serializer.serialize_map(Some(t.len()))?;
            for (k, v) in t.iter() {
                map.serialize_entry(k, &JsonValue{ inner: v })?;
            }
            map.end()
        } else {
            "UNIMPLEMENTED".serialize(serializer)
        }
    }
}
