# Releasing Scentkeep

The [Release workflow](.github/workflows/release.yml) runs when a `v*` tag is pushed. It builds Windows installers, a universal macOS disk image, and Linux packages for Arch, Debian, and Fedora plus an AppImage. It publishes the files to the GitHub Release for that tag.

## Prepare a version

1. Choose an unused version number. The tag must be `v` followed by that version, such as `v0.2.0`.
2. Set the same version in `package.json`, `src-tauri/tauri.conf.json`, and `src-tauri/Cargo.toml`. The workflow checks all three before building.
3. Update and commit both JavaScript lockfiles and the Rust lockfile:

   ```sh
   npm install
   bun install
   cargo check --manifest-path src-tauri/Cargo.toml
   ```

4. Run `npm run check`, then review and commit the changes. Push the commit to the branch you are releasing.

## Publish

Tag the release commit and push the tag:

```sh
git tag v0.2.0
git push origin HEAD
git push origin v0.2.0
```

Replace `0.2.0` with the version you prepared. Follow the run under **GitHub → Actions → Release** and check its assets on the **Releases** page. If a tagged build fails, fix the problem in a new commit and use a new version and tag; rerunning an old tag uses the workflow from that tagged commit.

The updater artifacts require the repository's `TAURI_SIGNING_PRIVATE_KEY` GitHub Actions secret. Keep a secure backup of the signing key: future updates must use the same key so installed apps can verify them. The corresponding public key is in `src-tauri/tauri.conf.json`.
