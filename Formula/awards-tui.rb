class AwardsTui < Formula
  desc "Look up and edit FORSCOM decorations (Ratatui TUI)"
  homepage "https://github.com/codythebeast89/awards-tui"
  url "https://github.com/codythebeast89/awards-tui/archive/refs/tags/v2.4.0.tar.gz"
  sha256 "656679fb3962286f7ffef7e730838aa6fdd71b704ce06706b1c506ca62369bd7"
  license "MIT"
  head "https://github.com/codythebeast89/awards-tui.git", branch: "master"

  depends_on "rust" => :build

  def install
    system "cargo", "install", "--locked", "--root", prefix, "--path", "crates/awards-tui"
  end

  test do
    assert_match "awards-tui", shell_output("#{bin}/awards-tui --help")
  end
end
