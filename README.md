# Scentkeep

Scentkeep is a desktop journal for your fragrance collection. Record each scent's brand, collection, year, perfumer, description, rating, and top, heart, and base notes. Browse or search your collection, maintain a catalogue of perfume notes, and attach image links to notes. Your collection is stored locally on your computer.

You can import and export fragrances and notes as CSV files. Scentkeep also checks for new releases in the app. On Windows, macOS, and Linux AppImage it can install a signed update and restart; for distribution packages on Linux it links to the release page.

On first launch, choose a backup folder. Scentkeep immediately writes `scentkeep-fragrances.csv` and `scentkeep-notes.csv` there, then refreshes both files after each change to fragrances or notes, including CSV imports. Open **Settings** to view or change the folder. If you choose **Later**, the app asks again the next time it opens. The two files can be imported into a fresh collection to restore the data.

## Install

Download the latest version from [GitHub Releases](https://github.com/reaperhound/scentkeep/releases/latest), then choose the file for your system:

| System | Download | Install |
| --- | --- | --- |
| Windows | `.exe` or `.msi` | Open the installer and follow its prompts. |
| macOS | `.dmg` | Open the disk image and drag Scentkeep to Applications. The universal build supports Intel and Apple silicon. |
| Arch Linux | `.pkg.tar.zst` | Run `sudo pacman -U ./scentkeep-*.pkg.tar.zst` in the download directory. |
| Debian or Ubuntu | `.deb` | Run `sudo apt install ./scentkeep_*_amd64.deb` in the download directory. |
| Fedora | `.rpm` | Run `sudo dnf install ./scentkeep-*.rpm` in the download directory. |
| Other Linux distributions | `.AppImage` | Run `chmod +x ./scentkeep_*.AppImage`, then open the file. |

Windows and macOS installers are currently unsigned by their respective platform certificate authorities, so your operating system may ask you to confirm that you want to open them.

## Update

Download the latest version from [GitHub Releases](https://github.com/reaperhound/scentkeep/releases/latest) and install it using the steps above.

## CSV import and export

Use **Import CSV** or **Export CSV** on the collection page for fragrances, or on the perfume notes page for the note catalogue. Export a file first to get a template that Excel can edit. Save it as UTF-8 CSV before importing; `.xlsx` and `.xls` workbooks are not supported.

Fragrance CSV columns are `collection,name,brand,year,audience,description,top_notes,heart_notes,base_notes,perfumer,bottle_rating,rating_count,image_url`. The three note columns contain JSON arrays, such as `["Rose","Musk"]`; leave a cell blank for no notes. Note catalogue CSV columns are `name,image_url`. Columns can be reordered, but the complete set of headers must be present with no extras.

Import validates the whole file before saving anything. Fragrances with the same brand and name, or notes with the same name, are skipped regardless of case or surrounding spaces. The result lists each skipped duplicate and its CSV row number.

## Development

The app uses Tauri 2, SvelteKit, TypeScript, and Rust. Install the [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) for your platform, then run:

```sh
bun install
bun run tauri dev
```

Release builds use npm. See [RELEASING.md](RELEASING.md) for versioning and publishing steps.
