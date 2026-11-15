use super::*;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

static BACKUP_LOCK: Mutex<()> = Mutex::new(());

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
    if let Some(folder) = configured_folder(app)? {
        write_backup(app, &folder).map_err(|error| {
            format!("Your change was saved, but the CSV backup failed: {error}")
        })?;
    }
    Ok(())
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
