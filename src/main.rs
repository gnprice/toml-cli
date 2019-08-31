use std::fs;
use std::str;
use toml_edit::{Document, value};

fn main() -> Result<(), Box<dyn std::error::Error + 'static>> {
    let path = "Cargo.toml";
    let data = fs::read(path)?;
    let data = str::from_utf8(&data)?;
    let mut doc = data.parse::<Document>()?;
    println!("{:#?}", doc["package"]["authors"][0]);
    doc["package"]["foo"] = value("bar");
    println!("{}", doc.to_string());
    Ok(())
}
