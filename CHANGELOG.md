# Changelog for toml-cli

## Unreleased

* `toml set` now behaves by default like `toml set --write`, without
  requiring an explicit `--write`.  This completes the transition from the
  legacy behavior, which was equivalent to `toml set --print`. (#7)

* **Breaking**: `toml set` now requires an explicit `--write` or `--print`.

  As a reminder, a future version will change the default behavior of
  `toml set` to `toml set --write`.  This transitional error behavior exists
  to help provide a smooth migration from the legacy behavior of `toml set`,
  which was equivalent to `toml set --print`. (#7)

* **Breaking**: A plain `toml set`, without `--write` or `--print`, now
  emits a warning message to stderr.

  As a reminder, a future version will change the default
  behavior of `toml set` from `toml set --print` to `toml set --write`.
  Always pass an explicit `--write` or `--print` to `toml set`. (#7)

* **Future breaking change**: A future version will change the default
  behavior of `toml set` from `toml set --print` to `toml set --write`.
  (Other versions before then may make plain `toml set` emit a warning,
  or fail with an error.)

  To ensure smooth upgrades in the future, always pass an explicit `--write`
  or `--print` to `toml set`. (#7)

* `toml set` accepts `--write`, to actually edit the file, or `--print` for
  the current default behavior. (#7)
* Started publishing release binaries for Linux.  These have also been
  backfilled for past releases, back to v0.2.1. (#3)
* Switched from `failure` as a dependency to `anyhow` and `thiserror`,
  its recommended successors.


## 0.2.3

* `toml get` on a missing key no longer panics.  This gives it the same
  behavior as `git config`: print nothing, and exit with failure. (#14)
* Fix query parse error on empty quoted key `""`,
  as in `toml get data.toml 'foo."".bar'`. (#20)


## 0.2.2

* New option `toml get -r` / `--raw`. (#19)


## 0.2.1

* **Breaking**: Previously `toml get` on a missing key would print "null"
  and exit with success.  Now it panics.  (The panic was filed as #14 and
  fixed in v0.2.3.  Since v0.2.3 there are also tests that would catch this
  sort of unplanned behavior change.)

* Update `lexical-core` dependency, fixing build on recent Rust toolchains. (#12)
* Update `toml_edit` dependency, fixing parse error on dotted keys. (#2)
* Update dependencies generally.
* Adjust so `cargo fmt` and `cargo clippy` are clean.


## 0.2.0

* **Breaking**: Change query format from `.foo.bar` to `foo.bar`,
  like TOML itself.


## 0.1.0

Initial release.

* `toml get`.
* `toml set`, just printing the modified version.
