use std::fs;
use toml::{Value};

fn main() -> Result<(), Box<dyn std::error::Error + 'static>> {
    let path = "Cargo.toml";
    let data = fs::read(path)?;
    let whole: Value = toml::from_slice(&data).unwrap();
    println!("{}", whole.get("package").unwrap().get("authors").unwrap()[0]);
    Ok(())
}
