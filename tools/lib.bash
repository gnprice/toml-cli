die() {
    echo "$1" >&2
    exit 1
}

get_version() {
    cargo run -q -- get Cargo.toml package.version --raw
}
