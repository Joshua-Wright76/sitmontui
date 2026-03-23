class Sitmon < Formula
  desc "Rust TUI situation monitor"
  homepage "https://github.com/Joshua-Wright76/sitmontui"
  url "https://github.com/Joshua-Wright76/sitmontui/archive/refs/tags/v0.1.0.tar.gz"
  sha256 "2cde48081c6ec5f45214c14ee11ba921a8d51b2792c750d709cf06cd85703188"
  license "MIT"
  head "https://github.com/Joshua-Wright76/sitmontui.git"

  def install
    bin.install "sitmon_cli" => "sitmon"
  end

end
