# Releasing lovelyDL

Releases are built by `.github/workflows/release.yml` when a `v*` tag is pushed. The workflow
makes a draft GitHub release with the Windows installer, the updater manifest and checksums.
Nothing is published until the draft is published by hand.

## One-time setup

### Updater keys

lovelyDL updates itself with Tauri's updater, which only installs updates signed with your
key. Generate the key pair once, on your own machine:

```bash
npx tauri signer generate -w lovelydl-updater.key
```

- Keep `lovelydl-updater.key` (the private key) and its password somewhere safe, outside the
  repository. `.gitignore` already refuses `*.key` files. Anyone with this key can push an
  update to every lovelyDL user.
- Add three repository secrets on GitHub (Settings › Secrets and variables › Actions):
  - `TAURI_SIGNING_PRIVATE_KEY`: the contents of `lovelydl-updater.key`
  - `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`: its password
  - `LOVELYDL_UPDATER_PUBKEY`: the contents of `lovelydl-updater.key.pub`

The public key is compiled into the app (`option_env!("LOVELYDL_UPDATER_PUBKEY")` in
`src-tauri/src/lib.rs`). A build without it works normally but says "Updates are not set up
for this build" when asked to update.

If the signing secrets are missing, the workflow still builds the installer but makes no
updater artifacts.

### Code signing (optional)

Unsigned installers work, but Windows SmartScreen warns about them ("Windows protected your
PC" → More info → Run anyway). To sign with an Authenticode certificate, add:

- `WINDOWS_CERTIFICATE`: the `.pfx` file, base64-encoded
- `WINDOWS_CERTIFICATE_PASSWORD`: its password

The workflow signs the installer and the app only when both exist.

## Cutting a release

1. Update `CHANGELOG.md` and the version in `package.json` (Tauri reads it from there) and
   in `Cargo.toml` (`workspace.package.version`).
2. Commit, then tag: `git tag v0.2.0 && git push origin v0.2.0`.
3. Wait for the workflow, check the draft release, publish it.

The app looks for updates at
`https://github.com/FlyShip09/lovelyDL/releases/latest/download/latest.json`, which the
workflow uploads with each release.
