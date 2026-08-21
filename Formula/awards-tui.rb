class AwardsTui < Formula
  desc "Look up and edit FORSCOM decorations (Ratatui TUI)"
  homepage "https://github.com/codythebeast89/awards-tui"
  license "MIT"
  head "https://github.com/codythebeast89/awards-tui.git", branch: "master"

  # Stable URL/sha256 filled after v2.1.0 tag is published.
  # url "https://github.com/codythebeast89/awards-tui/archive/refs/tags/v2.1.0.tar.gz"
  # sha256 "…"

  depends_on "rust" => :build

  def install
    system "cargo", "install", "--locked", "--root", prefix, "--path", "crates/awards-tui"
  end

  test do
    assert_match "awards-tui", shell_output("#{bin}/awards-tui --help")
  end
end
