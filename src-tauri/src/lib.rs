use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use tauri::Manager;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Fragrance {
    id: Option<i64>,
    collection: String,
    name: String,
    brand: String,
    year: i32,
    audience: String,
    description: String,
    top_notes: Vec<String>,
    heart_notes: Vec<String>,
    base_notes: Vec<String>,
    perfumer: String,
    bottle_rating: Option<f64>,
    rating_count: Option<i32>,
    image_url: String,
}

fn database(app: &tauri::AppHandle) -> Result<Connection, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let conn = Connection::open(dir.join("scentkeep.sqlite3")).map_err(|e| e.to_string())?;
    conn.execute_batch("CREATE TABLE IF NOT EXISTS fragrances (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        collection TEXT NOT NULL,
        name TEXT NOT NULL,
        brand TEXT NOT NULL,
        year INTEGER NOT NULL,
        audience TEXT NOT NULL,
        description TEXT NOT NULL,
        top_notes TEXT NOT NULL,
        heart_notes TEXT NOT NULL,
        base_notes TEXT NOT NULL,
        perfumer TEXT NOT NULL,
        bottle_rating REAL,
        rating_count INTEGER,
        image_url TEXT NOT NULL,
        created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
    );
    CREATE TABLE IF NOT EXISTS note_images (
        name TEXT PRIMARY KEY COLLATE NOCASE,
        image_data_url TEXT NOT NULL
    );").map_err(|e| e.to_string())?;
    Ok(conn)
}

fn validate_fragrance(fragrance: &Fragrance) -> Result<(), String> {
    if fragrance.name.trim().is_empty() || fragrance.brand.trim().is_empty() {
        return Err("Name and brand are required".into());
    }
    if !(1800..=2100).contains(&fragrance.year) {
        return Err("Enter a valid release year".into());
    }
    if fragrance.bottle_rating.is_some_and(|rating| !(0.0..=10.0).contains(&rating)) {
        return Err("Bottle rating must be between 0 and 10".into());
    }
    if fragrance.rating_count.is_some_and(|count| count < 0) {
        return Err("Rating count cannot be negative".into());
    }
    Ok(())
}

#[tauri::command]
fn save_fragrance(app: tauri::AppHandle, fragrance: Fragrance) -> Result<i64, String> {
    validate_fragrance(&fragrance)?;
    let conn = database(&app)?;
    conn.execute("INSERT INTO fragrances (collection, name, brand, year, audience, description, top_notes, heart_notes, base_notes, perfumer, bottle_rating, rating_count, image_url)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)", params![
        fragrance.collection.trim(), fragrance.name.trim(), fragrance.brand.trim(), fragrance.year,
        fragrance.audience, fragrance.description.trim(),
        serde_json::to_string(&fragrance.top_notes).map_err(|e| e.to_string())?,
        serde_json::to_string(&fragrance.heart_notes).map_err(|e| e.to_string())?,
        serde_json::to_string(&fragrance.base_notes).map_err(|e| e.to_string())?,
        fragrance.perfumer.trim(), fragrance.bottle_rating, fragrance.rating_count, fragrance.image_url.trim()
    ]).map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid())
}

#[tauri::command]
fn update_fragrance(app: tauri::AppHandle, fragrance: Fragrance) -> Result<(), String> {
    validate_fragrance(&fragrance)?;
    let id = fragrance.id.ok_or("Missing fragrance ID")?;
    let conn = database(&app)?;
    let changed = conn.execute("UPDATE fragrances SET collection = ?1, name = ?2, brand = ?3, year = ?4, audience = ?5, description = ?6, top_notes = ?7, heart_notes = ?8, base_notes = ?9, perfumer = ?10, bottle_rating = ?11, rating_count = ?12, image_url = ?13 WHERE id = ?14", params![
        fragrance.collection.trim(), fragrance.name.trim(), fragrance.brand.trim(), fragrance.year,
        fragrance.audience, fragrance.description.trim(),
        serde_json::to_string(&fragrance.top_notes).map_err(|e| e.to_string())?,
        serde_json::to_string(&fragrance.heart_notes).map_err(|e| e.to_string())?,
        serde_json::to_string(&fragrance.base_notes).map_err(|e| e.to_string())?,
        fragrance.perfumer.trim(), fragrance.bottle_rating, fragrance.rating_count, fragrance.image_url.trim(), id
    ]).map_err(|e| e.to_string())?;
    if changed == 0 { return Err("Fragrance not found".into()); }
    Ok(())
}

#[tauri::command]
fn delete_fragrance(app: tauri::AppHandle, id: i64) -> Result<(), String> {
    let conn = database(&app)?;
    let changed = conn.execute("DELETE FROM fragrances WHERE id = ?1", [id]).map_err(|e| e.to_string())?;
    if changed == 0 { return Err("Fragrance not found".into()); }
    Ok(())
}

#[tauri::command]
fn list_fragrances(app: tauri::AppHandle) -> Result<Vec<Fragrance>, String> {
    let conn = database(&app)?;
    let mut statement = conn.prepare("SELECT id, collection, name, brand, year, audience, description, top_notes, heart_notes, base_notes, perfumer, bottle_rating, rating_count, image_url FROM fragrances ORDER BY id DESC").map_err(|e| e.to_string())?;
    let rows = statement.query_map([], |row| {
        let top: String = row.get(7)?;
        let heart: String = row.get(8)?;
        let base: String = row.get(9)?;
        Ok(Fragrance {
            id: row.get(0)?, collection: row.get(1)?, name: row.get(2)?, brand: row.get(3)?,
            year: row.get(4)?, audience: row.get(5)?, description: row.get(6)?,
            top_notes: serde_json::from_str(&top).unwrap_or_default(),
            heart_notes: serde_json::from_str(&heart).unwrap_or_default(),
            base_notes: serde_json::from_str(&base).unwrap_or_default(),
            perfumer: row.get(10)?, bottle_rating: row.get(11)?, rating_count: row.get(12)?, image_url: row.get(13)?,
        })
    }).map_err(|e| e.to_string())?;
    rows.collect::<rusqlite::Result<Vec<_>>>().map_err(|e| e.to_string())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct NoteImage {
    name: String,
    image_data_url: String,
}

#[tauri::command]
fn list_note_images(app: tauri::AppHandle) -> Result<Vec<NoteImage>, String> {
    let conn = database(&app)?;
    let mut statement = conn.prepare("SELECT name, image_data_url FROM note_images")
        .map_err(|e| e.to_string())?;
    let rows = statement.query_map([], |row| Ok(NoteImage {
        name: row.get(0)?, image_data_url: row.get(1)?,
    })).map_err(|e| e.to_string())?;
    rows.collect::<rusqlite::Result<Vec<_>>>().map_err(|e| e.to_string())
}

#[tauri::command]
fn set_note_image(app: tauri::AppHandle, name: String, image_data_url: String) -> Result<(), String> {
    let name = name.trim();
    if name.is_empty() { return Err("Note name is required".into()); }
    if !image_data_url.starts_with("data:image/") || !image_data_url.contains(";base64,") {
        return Err("Select a valid image file".into());
    }
    if image_data_url.len() > 4_000_000 { return Err("Image is too large".into()); }
    let conn = database(&app)?;
    conn.execute("INSERT INTO note_images (name, image_data_url) VALUES (?1, ?2)
        ON CONFLICT(name) DO UPDATE SET image_data_url = excluded.image_data_url", params![name, image_data_url])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn remove_note_image(app: tauri::AppHandle, name: String) -> Result<(), String> {
    let conn = database(&app)?;
    conn.execute("DELETE FROM note_images WHERE name = ?1", [name.trim()]).map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![save_fragrance, update_fragrance, delete_fragrance, list_fragrances, list_note_images, set_note_image, remove_note_image])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
