# Releasing Scentkeep

The [Release workflow](.github/workflows/release.yml) runs when a `v*` tag is pushed. It builds Windows installers, a universal macOS disk image, and Linux packages for Arch, Debian, and Fedora plus an AppImage. It publishes the files to the GitHub Release for that tag.

## Prepare a version

From a clean working tree, choose an unused version and run:

```sh
python3 scripts/release.py prepare 0.2.0
```

The script uses the npm version pinned in `package.json`, matching CI. It updates the three version files and both JavaScript lockfiles, validates `npm ci`, refreshes the Rust lockfile, and runs `npm run check`. Review the diff and commit the release changes. If a check fails, fix it and rerun the failed check before committing.

Keep the npm pin when preparing releases: npm 11.6.2 can omit optional WebAssembly dependencies from this lockfile, producing a lockfile that npm 10 rejects.

## Publish

On `main`, with a clean working tree and the release commit in place, run:

```sh
python3 scripts/release.py publish
```

The script checks that all versions match, confirms the tag is unused and `main` is current, then asks before pushing the commit and tag. Follow the run under **GitHub → Actions → Release** and check its assets on the **Releases** page. If a tagged build fails, fix the problem in a new commit and use a new version and tag; rerunning an old tag uses the workflow from that tagged commit.

The updater artifacts require the repository's `TAURI_SIGNING_PRIVATE_KEY` GitHub Actions secret. Keep a secure backup of the signing key: future updates must use the same key so installed apps can verify them. The corresponding public key is in `src-tauri/tauri.conf.json`.
