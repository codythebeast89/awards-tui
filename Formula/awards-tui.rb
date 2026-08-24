class AwardsTui < Formula
  desc "Look up and edit FORSCOM decorations (Ratatui TUI)"
  homepage "https://github.com/codythebeast89/awards-tui"
  url "https://github.com/codythebeast89/awards-tui/archive/refs/tags/v2.2.0.tar.gz"
  sha256 "08144a207b42b232d8be67a3f153a418516d698fef721cfaab1ed36f4d521b5a"
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
