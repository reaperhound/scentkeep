# Tauri + SvelteKit + TypeScript

This template should help get you started developing with Tauri, SvelteKit and TypeScript in Vite.

## Recommended IDE Setup

[VS Code](https://code.visualstudio.com/) + [Svelte](https://marketplace.visualstudio.com/items?itemName=svelte.svelte-vscode) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer).

## Releases

The [release workflow](.github/workflows/release.yml) builds Windows (`.exe` and `.msi`), Linux (`.deb` and `.AppImage`), and universal macOS (`.dmg`) installers when a `v*` tag is pushed.

1. Set the same version in `package.json`, `src-tauri/tauri.conf.json`, and `src-tauri/Cargo.toml`.
2. Commit and push those version changes.
3. Tag that commit and push the tag, for example:

   ```sh
   git tag v0.2.0
   git push origin main v0.2.0
   ```

The workflow checks that all three versions match the tag, then publishes the installers to the corresponding GitHub Release. macOS builds are unsigned; distributing without Gatekeeper warnings requires Apple code signing and notarization. Windows builds are unsigned and may trigger SmartScreen warnings.
