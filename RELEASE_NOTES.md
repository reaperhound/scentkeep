# Scentkeep — changes since v0.1.8

Draft for the next release. These changes are not yet tagged or published.

## Highlights

- A redesigned collection interface brings clearer navigation, forms, and fragrance and note views.
- WebDAV backup can keep fragrance and note CSV files in an HTTPS folder. When that folder already contains backups, Scentkeep asks you to sync before uploading local changes.
- Automatic local and WebDAV backups now run in a background queue after changes are saved. Rapid changes are combined into the latest backup; unfinished backups retry on the next launch. Backup progress, failures, and a retry action are visible in the app, and closing during a backup offers a choice to wait or retry later.

## Improvements and fixes

- The brand/house field suggests brands already in your collection, with keyboard navigation.
- Save buttons and longer-running tasks provide clearer progress feedback.
- Parfumo note search no longer uses a blocking request on Linux. Arrow-key selection is visible, and smaller thumbnails and more efficient note-list updates make selection smoother.
- Parfumo session saving rejects Cloudflare verification pages and challenge-only cookies instead of reporting a false connection. The embedded Linux browser explicitly enables JavaScript, storage, and cookies, and Settings can clear its browser data before a retry.

## Known limitation

Parfumo's Cloudflare verification may still refuse the embedded browser. Scentkeep cannot force it to complete or save a usable session while the challenge remains. Notes can still be added manually by name.
