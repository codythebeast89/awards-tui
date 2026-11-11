class AwardsTui < Formula
  desc "Look up and edit FORSCOM decorations (Ratatui TUI)"
  homepage "https://github.com/codythebeast89/awards-tui"
  url "https://github.com/codythebeast89/awards-tui/archive/refs/tags/v2.4.1.tar.gz"
  sha256 "d0b773978b66f742e495cbe4c78a8f81ff394b04b925b7e1ae216fba45803d56"
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
