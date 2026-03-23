class Sitmon < Formula
  desc "Rust TUI situation monitor"
  homepage "https://github.com/Joshua-Wright76/sitmontui"
  license "MIT"
  head "https://github.com/Joshua-Wright76/sitmontui.git"

  on_macos do
    on_arm do
      url "https://github.com/Joshua-Wright76/sitmontui/releases/download/v0.1.0/sitmon_cli"
      sha256 "2ac73ff1271165051e228242c71c2b46ccb6cd0bd74b3bc8bde7c12bc9771149"
    end
    on_intel do
      url "https://github.com/Joshua-Wright76/sitmontui/releases/download/v0.1.0/sitmon_cli-x86_64-apple-darwin"
      sha256 "530eb6394db724040d294ff77ff08ece43e7c8325922fa53ecbf093ee5802437"
    end
  end

  def install
    bin.install "sitmon_cli" => "sitmon"
  end

  test do
    system "#{bin}/sitmon", "--help"
  end
end
