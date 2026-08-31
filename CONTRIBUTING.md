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
4. Confirm the commit email matches a verified GitHub email:
   ```bash
   git config --global user.email you@example.com
   ```

Older commits pushed before signing was enabled will remain unverified; only new signed commits get the badge.

If `gpg` prompts for a passphrase on every commit, use a GPG agent (for example `gpg-agent` with `pinentry`).
