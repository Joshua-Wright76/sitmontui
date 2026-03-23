class Sitmon < Formula
  desc "Rust TUI situation monitor"
  homepage "https://github.com/Joshua-Wright76/sitmontui"
  url "https://github.com/Joshua-Wright76/sitmontui/archive/refs/tags/v0.1.0.tar.gz"
  sha256 "TBD"
  license "MIT"
  head "https://github.com/Joshua-Wright76/sitmontui.git"

  def install
    bin.install "sitmon_cli" => "sitmon"
  end

  test do
    system "#{bin}/sitmon", "--version"
  end
end
