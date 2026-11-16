use super::*;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;
use serde::{Deserialize, Serialize};
use url::Url;

static BACKUP_LOCK: Mutex<()> = Mutex::new(());
const WEBDAV_SERVICE: &str = "com.scentkeep.app.webdav";
const WEBDAV_USER: &str = "backup";

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct WebdavBackup {
    url: String,
    username: String,
    #[serde(default)]
    needs_sync: bool,
}

fn webdav_setting_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(app.path().app_data_dir().map_err(|e| e.to_string())?.join("backup-webdav.json"))
}

fn configured_webdav(app: &tauri::AppHandle) -> Result<Option<WebdavBackup>, String> {
    let path = webdav_setting_path(app)?;
    if !path.exists() { return Ok(None); }
    let content = std::fs::read(path).map_err(|e| e.to_string())?;
    serde_json::from_slice(&content).map(Some).map_err(|e| e.to_string())
}

fn webdav_entry() -> Result<keyring::Entry, String> {
    keyring::Entry::new(WEBDAV_SERVICE, WEBDAV_USER).map_err(|e| e.to_string())
}

fn webdav_url(value: &str) -> Result<Url, String> {
    let mut url = Url::parse(value.trim()).map_err(|_| "Enter a valid WebDAV HTTPS folder URL")?;
    if url.scheme() != "https" || url.host_str().is_none() || !url.username().is_empty()
        || url.password().is_some() || url.query().is_some() || url.fragment().is_some() {
        return Err("Use an HTTPS WebDAV folder URL without credentials or query parameters".into());
    }
    if !url.path().ends_with('/') {
        url.set_path(&format!("{}/", url.path()));
    }
    Ok(url)
}

#[tauri::command]
pub(super) fn get_webdav_backup(app: tauri::AppHandle) -> Result<Option<WebdavBackup>, String> {
    configured_webdav(&app)
}

#[tauri::command]
pub(super) fn set_webdav_backup(app: tauri::AppHandle, url: String, username: String, password: String) -> Result<WebdavBackup, String> {
    let url = webdav_url(&url)?.to_string();
    let username = username.trim().to_owned();
    if username.is_empty() || password.is_empty() {
        return Err("Enter your WebDAV username and password".into());
    }
    let mut config = WebdavBackup { url, username, needs_sync: false };
    let _guard = BACKUP_LOCK.lock().map_err(|e| e.to_string())?;
    // A second installation must not replace an existing shared backup with its local data.
    if read_webdav(&config, &password)?.is_none() {
        let conn = database(&app)?;
        upload_webdav(&config, &password, csv_io::fragrance_csv(&conn)?, csv_io::note_csv(&conn)?)?;
    } else {
        config.needs_sync = true;
    }
    webdav_entry()?.set_password(&password)
        .map_err(|e| format!("Could not save the WebDAV password in the system keyring: {e}"))?;
    let path = webdav_setting_path(&app)?;
    std::fs::create_dir_all(path.parent().ok_or("Invalid settings path")?).map_err(|e| e.to_string())?;
    std::fs::write(path, serde_json::to_vec(&config).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    Ok(config)
}

#[tauri::command]
pub(super) fn disconnect_webdav_backup(app: tauri::AppHandle) -> Result<(), String> {
    let _guard = BACKUP_LOCK.lock().map_err(|e| e.to_string())?;
    let entry = webdav_entry()?;
    match entry.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => {},
        Err(e) => return Err(format!("Could not remove the WebDAV password: {e}")),
    }
    let path = webdav_setting_path(&app)?;
    if path.exists() { std::fs::remove_file(path).map_err(|e| e.to_string())?; }
    Ok(())
}

fn upload_webdav(config: &WebdavBackup, password: &str, fragrances: String, notes: String) -> Result<(), String> {
    let base = webdav_url(&config.url)?;
    let client = webdav_client()?;
    for (name, content) in [("scentkeep-fragrances.csv", fragrances), ("scentkeep-notes.csv", notes)] {
        let destination = base.join(name).map_err(|e| e.to_string())?;
        let response = client.put(destination)
            .basic_auth(&config.username, Some(password))
            .header(reqwest::header::CONTENT_TYPE, "text/csv; charset=utf-8")
            .body(content).send().map_err(|e| format!("WebDAV upload of {name} failed: {e}"))?;
        if !response.status().is_success() {
            return Err(format!("WebDAV upload of {name} failed: HTTP {}", response.status()));
        }
    }
    Ok(())
}

fn webdav_client() -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(15))
        .redirect(reqwest::redirect::Policy::none())
        .build().map_err(|e| e.to_string())
}

fn read_webdav(config: &WebdavBackup, password: &str) -> Result<Option<(String, String)>, String> {
    let base = webdav_url(&config.url)?;
    let client = webdav_client()?;
    let mut files = Vec::new();
    for name in ["scentkeep-fragrances.csv", "scentkeep-notes.csv"] {
        let destination = base.join(name).map_err(|e| e.to_string())?;
        let response = client.get(destination).basic_auth(&config.username, Some(password))
            .send().map_err(|e| format!("Could not read {name} from WebDAV: {e}"))?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            files.push(None);
        } else if response.status().is_success() {
            files.push(Some(response.text().map_err(|e| format!("Could not read {name}: {e}"))?));
        } else {
            return Err(format!("Could not read {name} from WebDAV: HTTP {}", response.status()));
        }
    }
    match (files.remove(0), files.remove(0)) {
        (None, None) => Ok(None),
        (Some(fragrances), Some(notes)) => Ok(Some((fragrances, notes))),
        _ => Err("The WebDAV folder contains only one Scentkeep backup file; both CSV files are required".into()),
    }
}

#[tauri::command]
pub(super) fn sync_from_webdav(app: tauri::AppHandle) -> Result<csv_io::SyncReport, String> {
    let mut config = configured_webdav(&app)?.ok_or("Connect WebDAV in Settings first")?;
    let _guard = BACKUP_LOCK.lock().map_err(|e| e.to_string())?;
    let password = webdav_entry()?.get_password()
        .map_err(|e| format!("Could not read the WebDAV password from the system keyring: {e}"))?;
    let (fragrances, notes) = read_webdav(&config, &password)?
        .ok_or("No Scentkeep CSV backups were found in this WebDAV folder")?;
    let report = csv_io::sync_backup(&mut database(&app)?, &fragrances, &notes)?;
    if config.needs_sync {
        config.needs_sync = false;
        std::fs::write(webdav_setting_path(&app)?, serde_json::to_vec(&config).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    }
    Ok(report)
}

fn setting_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("backup-folder.txt"))
}

fn configured_folder(app: &tauri::AppHandle) -> Result<Option<PathBuf>, String> {
    let path = setting_path(app)?;
    if !path.exists() {
        return Ok(None);
    }
    let value = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    Ok(Some(PathBuf::from(value)))
}

#[tauri::command]
pub(super) fn get_backup_folder(app: tauri::AppHandle) -> Result<Option<String>, String> {
    Ok(configured_folder(&app)?.map(|path| path.to_string_lossy().into_owned()))
}

#[tauri::command]
pub(super) fn set_backup_folder(app: tauri::AppHandle, folder: String) -> Result<(), String> {
    let folder = Path::new(folder.trim());
    if !folder.is_absolute() || !folder.is_dir() {
        return Err("Choose an existing backup folder".into());
    }
    // Create the first backup before remembering the folder, so an unwritable
    // location does not leave automatic backups apparently enabled.
    write_backup(&app, folder)?;
    let setting = setting_path(&app)?;
    std::fs::create_dir_all(setting.parent().ok_or("Invalid settings path")?)
        .map_err(|e| e.to_string())?;
    std::fs::write(setting, folder.to_string_lossy().as_bytes()).map_err(|e| e.to_string())
}

pub(super) fn after_change(app: &tauri::AppHandle) -> Result<(), String> {
    let mut errors = Vec::new();
    if let Some(folder) = configured_folder(app)? {
        if let Err(error) = write_backup(app, &folder) {
            errors.push(format!("local CSV backup failed: {error}"));
        }
    }
    if let Some(config) = configured_webdav(app)? {
        if config.needs_sync {
            errors.push("WebDAV upload is waiting for Sync from WebDAV on the collection page".into());
        } else {
        let result = (|| {
            let _guard = BACKUP_LOCK.lock().map_err(|e| e.to_string())?;
            let password = webdav_entry()?.get_password()
                .map_err(|e| format!("Could not read the WebDAV password from the system keyring: {e}"))?;
            let conn = database(app)?;
            upload_webdav(&config, &password, csv_io::fragrance_csv(&conn)?, csv_io::note_csv(&conn)?)
        })();
        if let Err(error) = result { errors.push(format!("WebDAV backup failed: {error}")); }
        }
    }
    if errors.is_empty() { Ok(()) } else { Err(format!("Your change was saved, but {}", errors.join("; "))) }
}

fn write_backup(app: &tauri::AppHandle, folder: &Path) -> Result<(), String> {
    let _guard = BACKUP_LOCK.lock().map_err(|e| e.to_string())?;
    if !folder.is_dir() {
        return Err(format!(
            "Backup folder is unavailable: {}",
            folder.display()
        ));
    }
    let conn = database(app)?;
    write_csv(
        folder.join("scentkeep-fragrances.csv"),
        csv_io::fragrance_csv(&conn)?,
    )?;
    write_csv(folder.join("scentkeep-notes.csv"), csv_io::note_csv(&conn)?)
}

fn write_csv(path: PathBuf, content: String) -> Result<(), String> {
    let temporary = path.with_extension(format!("csv.{}.tmp", std::process::id()));
    std::fs::write(&temporary, content).map_err(|e| e.to_string())?;
    if let Err(error) = std::fs::rename(&temporary, &path) {
        // Windows does not replace an existing destination with rename.
        if cfg!(windows) && path.exists() {
            std::fs::remove_file(&path).map_err(|e| e.to_string())?;
            std::fs::rename(&temporary, &path).map_err(|e| e.to_string())?;
        } else {
            let _ = std::fs::remove_file(&temporary);
            return Err(error.to_string());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::webdav_url;

    #[test]
    fn webdav_url_keeps_uploads_inside_the_selected_folder() {
        let base = webdav_url("https://example.com/dav/My%20Backups").unwrap();
        assert_eq!(base.join("scentkeep-notes.csv").unwrap().as_str(),
            "https://example.com/dav/My%20Backups/scentkeep-notes.csv");
        for invalid in ["http://example.com/dav/", "https://user:secret@example.com/dav/", "https://example.com/dav/?token=secret"] {
            assert!(webdav_url(invalid).is_err());
        }
    }
}
