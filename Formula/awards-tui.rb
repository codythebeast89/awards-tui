class AwardsTui < Formula
  desc "Look up and edit FORSCOM decorations (Ratatui TUI)"
  homepage "https://github.com/codythebeast89/awards-tui"
  url "https://github.com/codythebeast89/awards-tui/archive/refs/tags/v2.1.0.tar.gz"
  sha256 "ca0913c248ca219b8b5fadf76789f1a4460cde7fa7f38660cb3c6bb02e5a3585"
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
