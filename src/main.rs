use std::fs;
use std::path::PathBuf;
use std::str;
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
        Args::Get { path, query } => get(path, query),
    }
}

fn get(path: PathBuf, query: String) -> Result<(), Box<dyn std::error::Error>> {
    let data = fs::read(path)?;
    let data = str::from_utf8(&data)?;
    let doc = data.parse::<Document>()?;

    println!("{:#?}", doc["package"]["authors"][0]);

    /*
    doc["package"]["foo"] = value("bar");
    println!("{}", doc.to_string());
    */
    Ok(())
}
