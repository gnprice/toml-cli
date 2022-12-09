die() {
    echo "$1" >&2
    exit 1
}

get_version() {
    cargo run -q -- get Cargo.toml package.version --raw
}

# Set variables for color codes if color appropriate, or empty if not.
#
# Color is deemed appropriate just if stderr is a terminal.
#
# Variables set: reset, bold
prepare_colors() {
    local should_color=
    if [ -t 2 ]; then
        should_color=yes
    fi

    reset=
    bold=
    if [ -n "${should_color}" ]; then
        reset=$'\033'[0m
        bold=$'\033'[1m
    fi
    : "${reset}" "${bold}" # dummy use, to reassure shellcheck
}
