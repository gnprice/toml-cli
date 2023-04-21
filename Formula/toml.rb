class Toml < Formula
  desc "A command line utility written in Rust download, inspect and compare Substrate based chains WASM Runtimes"
  homepage "https://github.com/chevdor/toml-cli"
  url "https://github.com/chevdor/toml-cli/releases/download/v0.2.4/toml_macos_v0.2.4.tar.gz"
  sha256 "7cf5971646fda4e8edfd3723d05ff3d030e3d945dca72c32e1d2fc1f49529bb5"
  version "0.2.4"

  def install
    bin.install "toml"
  end
end
