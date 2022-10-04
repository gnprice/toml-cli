class Toml < Formula
  desc "A command line utility written in Rust download, inspect and compare Substrate based chains WASM Runtimes"
  homepage "https://github.com/chevdor/toml-cli"
  url "https://github.com/chevdor/toml-cli/releases/download/v0.2.2/toml_macos_v0.2.2.tar.gz"
  sha256 "daa2e7c8396e6bd00777f89e74197efb4e2e34b5bbae450cd98569fc7141cfd4"
  version "0.2.2"

  def install
    bin.install "toml"
  end
end
