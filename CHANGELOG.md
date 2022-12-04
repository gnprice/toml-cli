# Changelog for toml-cli

## Unreleased

* New option `toml get -r` / `--raw` (#19).


## 0.2.1

* Update dependencies, fixing build on recent Rust toolchains (#12).
* Adjust so `cargo fmt` and `cargo clippy` are clean.


## 0.2.0

* **Breaking**: Change query format from `.foo.bar` to `foo.bar`,
  like TOML itself.


## 0.1.0

Initial release.

* `toml get`.
* `toml set`, just printing the modified version.
