## Downloads

Download the binary for your OS from below:
- **Linux**
    - [Debian package]({{ DEBIAN_URL }})
- **MacOS**
    - [Archive]({{ MACOS_TGZ_URL }})
## Install

### From source

```
cargo install --git https://github.com/chevdor/toml-cli
```

### Linux
```
wget {{ DEBIAN_URL }} -O toml-cli.deb
sudo dpkg -i toml-cli.deb
toml --help
```

### MacOS

```
brew tap chevdor/toml https://github.com/chevdor/toml-cli
brew update
brew install chevdor/toml/toml
```

{{ CHANGELOG }}
