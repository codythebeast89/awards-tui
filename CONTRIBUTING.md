# Contributing

Thanks for helping improve awards-tui.

## Development

```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

Release builds are tagged `v*`; CI runs tests and clippy before publishing binaries.

## Signed commits

Commits on `master` should be **GPG-signed** so GitHub shows them as Verified.

1. Create or reuse a GPG key tied to an email on your GitHub account:
   ```bash
   gpg --full-generate-key
   gpg --list-secret-keys --keyid-format long
   ```
2. Upload the public key: **GitHub → Settings → SSH and GPG keys → New GPG key**
   ```bash
   gpg --armor --export <KEY_ID>
   ```
3. Enable signing locally:
   ```bash
   git config --global user.signingkey <KEY_ID>
   git config --global commit.gpgsign true
   ```
4. Set author identity to match your GitHub account (name and a verified email):
   ```bash
   git config --global user.name "Your GitHub Display Name"
   git config --global user.email you@example.com
   ```

### Already pushed unsigned commits

Commits pushed **before** signing was enabled stay **Unverified** on GitHub. That is expected — you do not need to rewrite history unless you specifically want a fully verified trail.

Example on this repo: the `v2.3.0` release bump commits (`fce0746`, `7d73a40`) were pushed unsigned; the first Verified commit is `eb25743` (this guide). Re-signing old commits would require `git rebase` / amend and a force-push, which is usually not worth it.

Only **new** signed commits get the Verified badge.

If `gpg` prompts for a passphrase on every commit, use a GPG agent (for example `gpg-agent` with `pinentry`).
