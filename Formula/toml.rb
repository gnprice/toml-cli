class Toml < Formula
  desc "A command line utility written in Rust download, inspect and compare Substrate based chains WASM Runtimes"
  homepage "https://github.com/chevdor/toml-cli"
  url "https://github.com/chevdor/toml-cli/releases/download/v0.2.1/toml_macos_v0.2.1.tar.gz"
  sha256 "f3cbf7c4c8c8d0f346080815cc8fe756b1d43977b51bbe7db81eec6b37d2f088"
  version "0.2.1"

  def install
    bin.install "toml"
  end
end
