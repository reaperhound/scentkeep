# Tauri + SvelteKit + TypeScript

## CSV import and export

Use **Import CSV** or **Export CSV** on the collection page for fragrances, or on the perfume notes page for the note catalogue. Export a file first to get a template that Excel can edit. Save it as UTF-8 CSV before importing; `.xlsx` and `.xls` workbooks are not supported.

Fragrance CSV columns are `collection,name,brand,year,audience,description,top_notes,heart_notes,base_notes,perfumer,bottle_rating,rating_count,image_url`. The three note columns contain JSON arrays, such as `["Rose","Musk"]`; leave a cell blank for no notes. Note catalogue CSV columns are `name,image_url`. Columns can be reordered, but the complete set of headers must be present with no extras.

Import validates the whole file before saving anything. Fragrances with the same brand and name, or notes with the same name, are skipped regardless of case or surrounding spaces. The result lists each skipped duplicate and its CSV row number.

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
