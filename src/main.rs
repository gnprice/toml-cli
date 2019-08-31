use std::fs;
use std::path::PathBuf;
use std::str;

use regex::Regex;
use structopt::StructOpt;
use toml_edit::{Document, value};

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
    println!("{:#?}", item);

    /*
    doc["package"]["foo"] = value("bar");
    println!("{}", doc.to_string());
    */
    Ok(())
}
