class AwardsTui < Formula
  desc "Look up and edit FORSCOM decorations (Ratatui TUI)"
  homepage "https://github.com/codythebeast89/awards-tui"
  url "https://github.com/codythebeast89/awards-tui/archive/refs/tags/v2.3.0.tar.gz"
  sha256 "3dbe2582fb879cdbc52f63cb53685e8f488525e0c12c96ad1b8fc3fd64f313f3"
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
