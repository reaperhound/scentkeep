use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use tauri::Manager;

mod accords;
mod backup;
mod csv_io;
mod parfumo;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Fragrance {
    id: Option<i64>,
    collection: String,
    name: String,
    brand: String,
    year: i32,
    audience: String,
    #[serde(default)]
    concentration: String,
    description: String,
    top_notes: Vec<String>,
    heart_notes: Vec<String>,
    base_notes: Vec<String>,
    perfumer: String,
    bottle_rating: Option<f64>,
    #[serde(default)]
    ratings: Ratings,
    rating_count: Option<i32>,
    image_url: String,
    #[serde(default = "default_shelf")]
    shelf: String,
    #[serde(default)]
    bottles_owned: i32,
    #[serde(default)]
    bottles_finished: i32,
    #[serde(default)]
    favourite: bool,
    #[serde(default)]
    price_bought: Option<f64>,
    #[serde(default)]
    current_price: Option<f64>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Ratings {
    scent: Option<f64>,
    longevity: Option<f64>,
    sillage: Option<f64>,
    bottle: Option<f64>,
    value_for_money: Option<f64>,
}

impl Ratings {
    fn values(&self) -> [Option<f64>; 5] {
        [
            self.scent,
            self.longevity,
            self.sillage,
            self.bottle,
            self.value_for_money,
        ]
    }
    fn overall(&self) -> Option<f64> {
        let values: Vec<_> = self.values().into_iter().flatten().collect();
        if values.is_empty() {
            None
        } else {
            Some((values.iter().sum::<f64>() / values.len() as f64 * 10.0).round() / 10.0)
        }
    }
}

#[cfg(test)]
#[test]
fn overall_rating_averages_available_categories() {
    let ratings = Ratings {
        scent: Some(8.0),
        longevity: Some(7.0),
        sillage: None,
        bottle: Some(9.0),
        value_for_money: Some(6.0),
    };
    assert_eq!(ratings.overall(), Some(7.5));
    assert_eq!(Ratings::default().overall(), None);
}

fn default_shelf() -> String {
    "Collection".into()
}

fn database(app: &tauri::AppHandle) -> Result<Connection, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let mut conn = Connection::open(dir.join("scentkeep.sqlite3")).map_err(|e| e.to_string())?;
    conn.execute_batch("PRAGMA foreign_keys = ON;")
        .map_err(|e| e.to_string())?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS fragrances (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        collection TEXT NOT NULL,
        name TEXT NOT NULL,
        brand TEXT NOT NULL,
        year INTEGER NOT NULL,
        audience TEXT NOT NULL,
        concentration TEXT NOT NULL DEFAULT '',
        description TEXT NOT NULL,
        top_notes TEXT NOT NULL,
        heart_notes TEXT NOT NULL,
        base_notes TEXT NOT NULL,
        perfumer TEXT NOT NULL,
        bottle_rating REAL,
        ratings TEXT NOT NULL DEFAULT '{}',
        rating_count INTEGER,
        image_url TEXT NOT NULL,
        shelf TEXT NOT NULL DEFAULT 'Collection',
        bottles_owned INTEGER NOT NULL DEFAULT 0,
        bottles_finished INTEGER NOT NULL DEFAULT 0,
        favourite INTEGER NOT NULL DEFAULT 0,
        price_bought REAL,
        current_price REAL,
        created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
    );
    CREATE TABLE IF NOT EXISTS note_images (
        name TEXT PRIMARY KEY COLLATE NOCASE,
        image_data_url TEXT NOT NULL,
        parfumo_url TEXT NOT NULL DEFAULT '',
        categories TEXT NOT NULL DEFAULT '[]'
    );",
    )
    .map_err(|e| e.to_string())?;
    let has_parfumo_url: bool = conn.query_row("SELECT EXISTS(SELECT 1 FROM pragma_table_info('note_images') WHERE name = 'parfumo_url')", [], |row| row.get(0)).map_err(|e| e.to_string())?;
    if !has_parfumo_url {
        conn.execute(
            "ALTER TABLE note_images ADD COLUMN parfumo_url TEXT NOT NULL DEFAULT ''",
            [],
        )
        .map_err(|e| e.to_string())?;
    }
    let has_categories: bool = conn.query_row("SELECT EXISTS(SELECT 1 FROM pragma_table_info('note_images') WHERE name = 'categories')", [], |row| row.get(0)).map_err(|e| e.to_string())?;
    if !has_categories {
        conn.execute(
            "ALTER TABLE note_images ADD COLUMN categories TEXT NOT NULL DEFAULT '[]'",
            [],
        )
        .map_err(|e| e.to_string())?;
    }
    let columns = [
        ("shelf", "TEXT NOT NULL DEFAULT 'Collection'"),
        ("bottles_owned", "INTEGER NOT NULL DEFAULT 0"),
        ("bottles_finished", "INTEGER NOT NULL DEFAULT 0"),
        ("favourite", "INTEGER NOT NULL DEFAULT 0"),
        ("price_bought", "REAL"),
        ("current_price", "REAL"),
        ("concentration", "TEXT NOT NULL DEFAULT ''"),
        ("ratings", "TEXT NOT NULL DEFAULT '{}'"),
    ];
    for (name, definition) in columns {
        let exists: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM pragma_table_info('fragrances') WHERE name = ?1)",
                [name],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;
        if !exists {
            conn.execute(
                &format!("ALTER TABLE fragrances ADD COLUMN {name} {definition}"),
                [],
            )
            .map_err(|e| e.to_string())?;
        }
    }
    ensure_fragrance_notes_schema(&mut conn)?;
    Ok(conn)
}

fn ensure_fragrance_notes_schema(conn: &mut Connection) -> Result<(), String> {
    let exists: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'fragrance_notes')",
            [],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    if exists {
        return Ok(());
    }

    let transaction = conn
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .map_err(|e| e.to_string())?;
    transaction
        .execute_batch(
            "CREATE TABLE fragrance_notes (
                fragrance_id INTEGER NOT NULL REFERENCES fragrances(id) ON DELETE CASCADE,
                note_name TEXT NOT NULL COLLATE NOCASE REFERENCES note_images(name) ON UPDATE CASCADE ON DELETE CASCADE,
                layer TEXT NOT NULL CHECK (layer IN ('top', 'heart', 'base')),
                position INTEGER NOT NULL CHECK (position >= 0),
                PRIMARY KEY (fragrance_id, layer, position)
            );
            CREATE INDEX fragrance_notes_note_name_idx ON fragrance_notes(note_name COLLATE NOCASE);",
        )
        .map_err(|e| e.to_string())?;

    let fragrances = {
        let mut statement = transaction
            .prepare("SELECT id, top_notes, heart_notes, base_notes FROM fragrances")
            .map_err(|e| e.to_string())?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|e| e.to_string())?
    };
    for (id, top, heart, base) in fragrances {
        let layers = [top, heart, base]
            .map(|json| serde_json::from_str::<Vec<String>>(&json).map_err(|e| e.to_string()))
            .into_iter()
            .collect::<Result<Vec<_>, _>>()?;
        replace_fragrance_notes(&transaction, id, [&layers[0], &layers[1], &layers[2]])?;
    }
    transaction.commit().map_err(|e| e.to_string())
}

fn replace_fragrance_notes(
    conn: &Connection,
    fragrance_id: i64,
    layers: [&Vec<String>; 3],
) -> Result<(), String> {
    conn.execute(
        "DELETE FROM fragrance_notes WHERE fragrance_id = ?1",
        [fragrance_id],
    )
    .map_err(|e| e.to_string())?;
    for (layer, notes) in ["top", "heart", "base"].into_iter().zip(layers) {
        for (position, note) in notes.iter().enumerate() {
            let note = note.trim();
            if note.is_empty() {
                continue;
            }
            conn.execute(
                "INSERT OR IGNORE INTO note_images (name, image_data_url) VALUES (?1, '')",
                [note],
            )
            .map_err(|e| e.to_string())?;
            conn.execute(
                "INSERT INTO fragrance_notes (fragrance_id, note_name, layer, position) VALUES (?1, ?2, ?3, ?4)",
                params![fragrance_id, note, layer, position as i64],
            )
            .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

fn validate_fragrance(fragrance: &Fragrance) -> Result<(), String> {
    if fragrance.name.trim().is_empty() || fragrance.brand.trim().is_empty() {
        return Err("Name and brand are required".into());
    }
    if !(1800..=2100).contains(&fragrance.year) {
        return Err("Enter a valid release year".into());
    }
    if fragrance
        .bottle_rating
        .is_some_and(|rating| !(0.0..=10.0).contains(&rating))
    {
        return Err("Bottle rating must be between 0 and 10".into());
    }
    if fragrance
        .ratings
        .values()
        .into_iter()
        .flatten()
        .any(|rating| !rating.is_finite() || !(0.0..=10.0).contains(&rating))
    {
        return Err("Ratings must be between 0 and 10".into());
    }
    if fragrance.rating_count.is_some_and(|count| count < 0) {
        return Err("Rating count cannot be negative".into());
    }
    if !["Collection", "Wishlist", "Empties"].contains(&fragrance.shelf.as_str()) {
        return Err("Choose a valid category".into());
    }
    if fragrance.concentration.len() > 80 {
        return Err("Concentration is too long".into());
    }
    if fragrance.bottles_owned < 0 || fragrance.bottles_finished < 0 {
        return Err("Bottle counts cannot be negative".into());
    }
    if [fragrance.price_bought, fragrance.current_price]
        .iter()
        .flatten()
        .any(|price| !price.is_finite() || *price < 0.0)
    {
        return Err("Prices must be nonnegative numbers".into());
    }
    if !fragrance.image_url.trim().is_empty() {
        validate_image_url(&fragrance.image_url)?;
    }
    Ok(())
}

fn validate_image_url(value: &str) -> Result<(), String> {
    let parsed =
        url::Url::parse(value.trim()).map_err(|_| "Enter a valid image link".to_string())?;
    if !matches!(parsed.scheme(), "http" | "https") || parsed.host_str().is_none() {
        return Err("Image links must start with http:// or https://".into());
    }
    Ok(())
}

fn ensure_unique_fragrance(
    conn: &Connection,
    fragrance: &Fragrance,
    except_id: Option<i64>,
) -> Result<(), String> {
    let name = fragrance.name.trim().to_lowercase();
    let brand = fragrance.brand.trim().to_lowercase();
    let mut statement = conn
        .prepare("SELECT id, name, brand FROM fragrances")
        .map_err(|e| e.to_string())?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .map_err(|e| e.to_string())?;
    for row in rows {
        let (id, saved_name, saved_brand) = row.map_err(|e| e.to_string())?;
        if Some(id) != except_id
            && saved_name.trim().to_lowercase() == name
            && saved_brand.trim().to_lowercase() == brand
        {
            return Err("This fragrance is already in your collection".into());
        }
    }
    Ok(())
}

#[tauri::command]
fn save_fragrance(app: tauri::AppHandle, fragrance: Fragrance) -> Result<i64, String> {
    validate_fragrance(&fragrance)?;
    let mut conn = database(&app)?;
    let transaction = conn
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .map_err(|e| e.to_string())?;
    ensure_unique_fragrance(&transaction, &fragrance, None)?;
    let id = insert_fragrance(&transaction, &fragrance)?;
    transaction.commit().map_err(|e| e.to_string())?;
    backup::after_change(&app)?;
    Ok(id)
}

fn insert_fragrance(conn: &Connection, fragrance: &Fragrance) -> Result<i64, String> {
    conn.execute("INSERT INTO fragrances (collection, name, brand, year, audience, description, top_notes, heart_notes, base_notes, perfumer, bottle_rating, rating_count, image_url, shelf, bottles_owned, bottles_finished, favourite, price_bought, current_price, concentration, ratings)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21)", params![
        fragrance.collection.trim(), fragrance.name.trim(), fragrance.brand.trim(), fragrance.year,
        fragrance.audience, fragrance.description.trim(),
        serde_json::to_string(&fragrance.top_notes).map_err(|e| e.to_string())?,
        serde_json::to_string(&fragrance.heart_notes).map_err(|e| e.to_string())?,
        serde_json::to_string(&fragrance.base_notes).map_err(|e| e.to_string())?,
        fragrance.perfumer.trim(), fragrance.ratings.overall().or(fragrance.bottle_rating), fragrance.rating_count, fragrance.image_url.trim(), fragrance.shelf, fragrance.bottles_owned, fragrance.bottles_finished, fragrance.favourite, fragrance.price_bought, fragrance.current_price, fragrance.concentration.trim(), serde_json::to_string(&fragrance.ratings).map_err(|e| e.to_string())?
    ]).map_err(|e| e.to_string())?;
    let id = conn.last_insert_rowid();
    replace_fragrance_notes(
        conn,
        id,
        [
            &fragrance.top_notes,
            &fragrance.heart_notes,
            &fragrance.base_notes,
        ],
    )?;
    Ok(id)
}

#[tauri::command]
fn update_fragrance(app: tauri::AppHandle, fragrance: Fragrance) -> Result<(), String> {
    validate_fragrance(&fragrance)?;
    let id = fragrance.id.ok_or("Missing fragrance ID")?;
    let mut conn = database(&app)?;
    let transaction = conn
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .map_err(|e| e.to_string())?;
    ensure_unique_fragrance(&transaction, &fragrance, Some(id))?;
    let changed = transaction.execute("UPDATE fragrances SET collection = ?1, name = ?2, brand = ?3, year = ?4, audience = ?5, description = ?6, top_notes = ?7, heart_notes = ?8, base_notes = ?9, perfumer = ?10, bottle_rating = ?11, rating_count = ?12, image_url = ?13, shelf = ?14, bottles_owned = ?15, bottles_finished = ?16, favourite = ?17, price_bought = ?18, current_price = ?19, concentration = ?20, ratings = ?22 WHERE id = ?21", params![
        fragrance.collection.trim(), fragrance.name.trim(), fragrance.brand.trim(), fragrance.year,
        fragrance.audience, fragrance.description.trim(),
        serde_json::to_string(&fragrance.top_notes).map_err(|e| e.to_string())?,
        serde_json::to_string(&fragrance.heart_notes).map_err(|e| e.to_string())?,
        serde_json::to_string(&fragrance.base_notes).map_err(|e| e.to_string())?,
        fragrance.perfumer.trim(), fragrance.ratings.overall().or(fragrance.bottle_rating), fragrance.rating_count, fragrance.image_url.trim(), fragrance.shelf, fragrance.bottles_owned, fragrance.bottles_finished, fragrance.favourite, fragrance.price_bought, fragrance.current_price, fragrance.concentration.trim(), id, serde_json::to_string(&fragrance.ratings).map_err(|e| e.to_string())?
    ]).map_err(|e| e.to_string())?;
    if changed == 0 {
        return Err("Fragrance not found".into());
    }
    replace_fragrance_notes(
        &transaction,
        id,
        [
            &fragrance.top_notes,
            &fragrance.heart_notes,
            &fragrance.base_notes,
        ],
    )?;
    transaction.commit().map_err(|e| e.to_string())?;
    backup::after_change(&app)
}

#[tauri::command]
fn delete_fragrance(app: tauri::AppHandle, id: i64) -> Result<(), String> {
    let conn = database(&app)?;
    let changed = conn
        .execute("DELETE FROM fragrances WHERE id = ?1", [id])
        .map_err(|e| e.to_string())?;
    if changed == 0 {
        return Err("Fragrance not found".into());
    }
    backup::after_change(&app)
}

#[tauri::command]
fn list_fragrances(app: tauri::AppHandle) -> Result<Vec<Fragrance>, String> {
    let conn = database(&app)?;
    load_fragrances(&conn)
}

fn load_fragrances(conn: &Connection) -> Result<Vec<Fragrance>, String> {
    let mut note_layers: HashMap<i64, [Vec<String>; 3]> = HashMap::new();
    {
        let mut statement = conn
            .prepare(
                "SELECT fn.fragrance_id, fn.layer, ni.name
                 FROM fragrance_notes fn
                 JOIN note_images ni ON ni.name = fn.note_name
                 ORDER BY fn.fragrance_id, CASE fn.layer WHEN 'top' THEN 0 WHEN 'heart' THEN 1 ELSE 2 END, fn.position",
            )
            .map_err(|e| e.to_string())?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })
            .map_err(|e| e.to_string())?;
        for row in rows {
            let (id, layer, name) = row.map_err(|e| e.to_string())?;
            let index = match layer.as_str() {
                "top" => 0,
                "heart" => 1,
                _ => 2,
            };
            note_layers.entry(id).or_default()[index].push(name);
        }
    }
    let mut statement = conn.prepare("SELECT id, collection, name, brand, year, audience, description, perfumer, bottle_rating, rating_count, image_url, shelf, bottles_owned, bottles_finished, favourite, price_bought, current_price, concentration, ratings FROM fragrances ORDER BY id DESC").map_err(|e| e.to_string())?;
    let rows = statement
        .query_map([], |row| {
            let id = row.get::<_, i64>(0)?;
            let layers = note_layers.remove(&id).unwrap_or_default();
            Ok(Fragrance {
                id: Some(id),
                collection: row.get(1)?,
                name: row.get(2)?,
                brand: row.get(3)?,
                year: row.get(4)?,
                audience: row.get(5)?,
                concentration: row.get(17)?,
                description: row.get(6)?,
                top_notes: layers[0].clone(),
                heart_notes: layers[1].clone(),
                base_notes: layers[2].clone(),
                perfumer: row.get(7)?,
                bottle_rating: row.get(8)?,
                ratings: serde_json::from_str(&row.get::<_, String>(18)?).unwrap_or_default(),
                rating_count: row.get(9)?,
                image_url: row.get(10)?,
                shelf: row.get(11)?,
                bottles_owned: row.get(12)?,
                bottles_finished: row.get(13)?,
                favourite: row.get(14)?,
                price_bought: row.get(15)?,
                current_price: row.get(16)?,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|e| e.to_string())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct NoteImage {
    name: String,
    image_url: String,
    parfumo_url: String,
    categories: Vec<String>,
    usage_count: usize,
}

#[tauri::command]
fn list_note_images(app: tauri::AppHandle) -> Result<Vec<NoteImage>, String> {
    let conn = database(&app)?;
    let upgraded = parfumo::upgrade_note_images(&conn)?;
    let cleaned = parfumo::clean_saved_note_categories(&conn)?;
    let notes = load_note_images(&conn)?;
    if upgraded + cleaned > 0 {
        backup::after_change(&app)?;
    }
    Ok(notes)
}

fn load_note_images(conn: &Connection) -> Result<Vec<NoteImage>, String> {
    let mut statement = conn
        .prepare(
            "SELECT ni.name, ni.image_data_url, ni.parfumo_url, ni.categories,
                    COUNT(DISTINCT fn.fragrance_id)
             FROM note_images ni
             LEFT JOIN fragrance_notes fn ON fn.note_name = ni.name
             GROUP BY ni.name, ni.image_data_url, ni.parfumo_url, ni.categories",
        )
        .map_err(|e| e.to_string())?;
    let rows = statement
        .query_map([], |row| {
            Ok(NoteImage {
                name: row.get(0)?,
                image_url: row.get(1)?,
                parfumo_url: row.get(2)?,
                categories: serde_json::from_str(&row.get::<_, String>(3)?).unwrap_or_default(),
                usage_count: row.get(4)?,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|e| e.to_string())
}

pub(crate) fn load_note_image(conn: &Connection, name: &str) -> Result<NoteImage, String> {
    conn.query_row(
        "SELECT ni.name, ni.image_data_url, ni.parfumo_url, ni.categories,
                COUNT(DISTINCT fn.fragrance_id)
         FROM note_images ni
         LEFT JOIN fragrance_notes fn ON fn.note_name = ni.name
         WHERE ni.name = ?1
         GROUP BY ni.name, ni.image_data_url, ni.parfumo_url, ni.categories",
        [name],
        |row| {
            Ok(NoteImage {
                name: row.get(0)?,
                image_url: row.get(1)?,
                parfumo_url: row.get(2)?,
                categories: serde_json::from_str(&row.get::<_, String>(3)?).unwrap_or_default(),
                usage_count: row.get(4)?,
            })
        },
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
fn create_note(app: tauri::AppHandle, name: String) -> Result<NoteImage, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("Enter a note name".into());
    }
    let mut conn = database(&app)?;
    let transaction = conn
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .map_err(|e| e.to_string())?;
    ensure_unique_note(&transaction, name, None)?;
    transaction
        .execute(
            "INSERT INTO note_images (name, image_data_url) VALUES (?1, '')",
            [name],
        )
        .map_err(|e| {
            if e.to_string().contains("UNIQUE") {
                "That note already exists".into()
            } else {
                e.to_string()
            }
        })?;
    transaction.commit().map_err(|e| e.to_string())?;
    backup::after_change(&app)?;
    load_note_image(&conn, name)
}

fn ensure_unique_note(
    conn: &Connection,
    name: &str,
    except_name: Option<&str>,
) -> Result<(), String> {
    let name = name.trim().to_lowercase();
    let mut statement = conn
        .prepare("SELECT name FROM note_images")
        .map_err(|e| e.to_string())?;
    let rows = statement
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|e| e.to_string())?;
    for row in rows {
        let saved_name = row.map_err(|e| e.to_string())?;
        if saved_name.trim().to_lowercase() == name
            && !except_name.is_some_and(|old| saved_name.eq_ignore_ascii_case(old.trim()))
        {
            return Err("That note already exists".into());
        }
    }
    Ok(())
}

fn rewrite_fragrance_notes(
    conn: &Connection,
    old_name: &str,
    new_name: Option<&str>,
) -> Result<(), String> {
    let mut statement = conn
        .prepare("SELECT id, top_notes, heart_notes, base_notes FROM fragrances")
        .map_err(|e| e.to_string())?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })
        .map_err(|e| e.to_string())?;
    let mut changed = Vec::new();
    for row in rows {
        let (id, top, heart, base) = row.map_err(|e| e.to_string())?;
        let mut layers = [top, heart, base]
            .map(|json| serde_json::from_str::<Vec<String>>(&json).map_err(|e| e.to_string()))
            .into_iter()
            .collect::<Result<Vec<_>, _>>()?;
        if layers
            .iter()
            .any(|layer| layer.iter().any(|note| note.eq_ignore_ascii_case(old_name)))
        {
            for layer in &mut layers {
                if let Some(replacement) = new_name {
                    for note in layer.iter_mut() {
                        if note.eq_ignore_ascii_case(old_name) {
                            *note = replacement.to_string();
                        }
                    }
                } else {
                    layer.retain(|note| !note.eq_ignore_ascii_case(old_name));
                }
            }
            changed.push((id, layers));
        }
    }
    drop(statement);
    for (id, layers) in changed {
        conn.execute(
            "UPDATE fragrances SET top_notes = ?1, heart_notes = ?2, base_notes = ?3 WHERE id = ?4",
            params![
                serde_json::to_string(&layers[0]).map_err(|e| e.to_string())?,
                serde_json::to_string(&layers[1]).map_err(|e| e.to_string())?,
                serde_json::to_string(&layers[2]).map_err(|e| e.to_string())?,
                id
            ],
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn rename_note(
    app: tauri::AppHandle,
    old_name: String,
    new_name: String,
) -> Result<NoteImage, String> {
    let old_name = old_name.trim();
    let new_name = new_name.trim();
    if new_name.is_empty() {
        return Err("Enter a note name".into());
    }
    let mut conn = database(&app)?;
    let transaction = conn.transaction().map_err(|e| e.to_string())?;
    ensure_unique_note(&transaction, new_name, Some(old_name))?;
    let changed = transaction
        .execute(
            "UPDATE note_images SET name = ?1 WHERE name = ?2",
            params![new_name, old_name],
        )
        .map_err(|e| {
            if e.to_string().contains("UNIQUE") {
                "That note already exists".into()
            } else {
                e.to_string()
            }
        })?;
    if changed == 0 {
        return Err("Note not found".into());
    }
    rewrite_fragrance_notes(&transaction, old_name, Some(new_name))?;
    transaction.commit().map_err(|e| e.to_string())?;
    backup::after_change(&app)?;
    load_note_image(&conn, new_name)
}

#[tauri::command]
fn delete_note(app: tauri::AppHandle, name: String) -> Result<(), String> {
    let mut conn = database(&app)?;
    let transaction = conn.transaction().map_err(|e| e.to_string())?;
    let changed = transaction
        .execute("DELETE FROM note_images WHERE name = ?1", [name.trim()])
        .map_err(|e| e.to_string())?;
    if changed == 0 {
        return Err("Note not found".into());
    }
    rewrite_fragrance_notes(&transaction, name.trim(), None)?;
    transaction.commit().map_err(|e| e.to_string())?;
    backup::after_change(&app)
}

#[tauri::command]
fn set_note_image(
    app: tauri::AppHandle,
    name: String,
    image_url: String,
) -> Result<NoteImage, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("Note name is required".into());
    }
    validate_image_url(&image_url)?;
    let conn = database(&app)?;
    let changed = conn
        .execute(
            "UPDATE note_images SET image_data_url = ?2 WHERE name = ?1",
            params![name, image_url.trim()],
        )
        .map_err(|e| e.to_string())?;
    if changed == 0 {
        return Err("Note not found".into());
    }
    backup::after_change(&app)?;
    load_note_image(&conn, name)
}

#[tauri::command]
fn remove_note_image(app: tauri::AppHandle, name: String) -> Result<NoteImage, String> {
    let conn = database(&app)?;
    let changed = conn
        .execute(
            "UPDATE note_images SET image_data_url = '' WHERE name = ?1",
            [name.trim()],
        )
        .map_err(|e| e.to_string())?;
    if changed == 0 {
        return Err("Note not found".into());
    }
    backup::after_change(&app)?;
    load_note_image(&conn, name.trim())
}

#[tauri::command]
fn set_note_categories(
    app: tauri::AppHandle,
    name: String,
    categories: Vec<String>,
) -> Result<NoteImage, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("Note name is required".into());
    }
    let categories = parfumo::clean_categories(name, categories);
    if categories.len() > 20
        || categories
            .iter()
            .any(|category| category.chars().count() > 80)
    {
        return Err("Enter up to 20 categories, each no longer than 80 characters".into());
    }
    let conn = database(&app)?;
    let changed = conn
        .execute(
            "UPDATE note_images SET categories = ?2 WHERE name = ?1",
            params![
                name,
                serde_json::to_string(&categories).map_err(|e| e.to_string())?
            ],
        )
        .map_err(|e| e.to_string())?;
    if changed == 0 {
        return Err("Note not found".into());
    }
    backup::after_change(&app)?;
    load_note_image(&conn, name)
}

#[tauri::command]
fn can_install_update() -> bool {
    #[cfg(target_os = "linux")]
    {
        std::env::var_os("APPIMAGE").is_some()
    }
    #[cfg(not(target_os = "linux"))]
    {
        true
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            backup::resume_backups(app.handle());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            save_fragrance,
            update_fragrance,
            delete_fragrance,
            list_fragrances,
            accords::get_fragrance_accords,
            list_note_images,
            set_note_image,
            remove_note_image,
            set_note_categories,
            create_note,
            rename_note,
            delete_note,
            can_install_update,
            backup::get_backup_folder,
            backup::set_backup_folder,
            backup::get_webdav_backup,
            backup::set_webdav_backup,
            backup::disconnect_webdav_backup,
            backup::sync_from_webdav,
            backup::backup_status,
            backup::retry_backup,
            csv_io::export_fragrances_csv,
            csv_io::import_fragrances_csv,
            csv_io::export_notes_csv,
            csv_io::import_notes_csv,
            parfumo::parfumo_status,
            parfumo::open_parfumo_login,
            parfumo::save_parfumo_session,
            parfumo::reset_parfumo_browser,
            parfumo::disconnect_parfumo,
            parfumo::search_parfumo_notes,
            parfumo::save_parfumo_note,
            parfumo::refresh_parfumo_note_metadata
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renaming_and_deleting_notes_updates_fragrance_layers() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE fragrances (id INTEGER PRIMARY KEY, top_notes TEXT, heart_notes TEXT, base_notes TEXT);
            INSERT INTO fragrances VALUES (1, '[\"Rose\",\"Apple\"]', '[\"rose\"]', '[\"Musk\"]');").unwrap();

        rewrite_fragrance_notes(&conn, "rose", Some("Rose absolute")).unwrap();
        let layers: (String, String, String) = conn
            .query_row(
                "SELECT top_notes, heart_notes, base_notes FROM fragrances WHERE id = 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(layers.0, r#"["Rose absolute","Apple"]"#);
        assert_eq!(layers.1, r#"["Rose absolute"]"#);
        assert_eq!(layers.2, r#"["Musk"]"#);

        rewrite_fragrance_notes(&conn, "Rose absolute", None).unwrap();
        let layers: (String, String) = conn
            .query_row(
                "SELECT top_notes, heart_notes FROM fragrances WHERE id = 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(layers.0, r#"["Apple"]"#);
        assert_eq!(layers.1, "[]");
    }

    #[test]
    fn image_urls_require_web_links() {
        assert!(validate_image_url("https://example.com/image.jpg").is_ok());
        assert!(validate_image_url("http://example.com/image.png").is_ok());
        assert!(validate_image_url("data:image/png;base64,abc").is_err());
        assert!(validate_image_url("file:///tmp/image.png").is_err());
        assert!(validate_image_url("not a link").is_err());
    }

    #[test]
    fn note_usage_counts_each_fragrance_once() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE fragrances (id INTEGER PRIMARY KEY, top_notes TEXT, heart_notes TEXT, base_notes TEXT);
            CREATE TABLE note_images (name TEXT PRIMARY KEY COLLATE NOCASE, image_data_url TEXT NOT NULL, parfumo_url TEXT NOT NULL DEFAULT '', categories TEXT NOT NULL DEFAULT '[]');
            CREATE TABLE fragrance_notes (fragrance_id INTEGER NOT NULL, note_name TEXT NOT NULL COLLATE NOCASE, layer TEXT NOT NULL, position INTEGER NOT NULL, PRIMARY KEY (fragrance_id, layer, position));
            INSERT INTO fragrances VALUES (1, '[\"Rose\",\"rose\"]', '[\"ROSE\"]', '[]');
            INSERT INTO fragrances VALUES (2, '[\"Rose\"]', '[]', '[]');
            INSERT INTO note_images (name, image_data_url) VALUES ('Rose', '');
            INSERT INTO note_images (name, image_data_url) VALUES ('Jasmine', '');").unwrap();
        conn.execute_batch(
            "INSERT INTO fragrance_notes VALUES (1, 'Rose', 'top', 0);
            INSERT INTO fragrance_notes VALUES (1, 'rose', 'top', 1);
            INSERT INTO fragrance_notes VALUES (1, 'ROSE', 'heart', 0);
            INSERT INTO fragrance_notes VALUES (2, 'Rose', 'top', 0);",
        )
        .unwrap();

        let notes = load_note_images(&conn).unwrap();
        assert_eq!(
            notes
                .iter()
                .find(|note| note.name == "Rose")
                .unwrap()
                .usage_count,
            2
        );
        assert_eq!(
            notes
                .iter()
                .find(|note| note.name == "Jasmine")
                .unwrap()
                .usage_count,
            0
        );
    }

    #[test]
    fn fragrance_note_schema_backfills_existing_json_once() {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON;
            CREATE TABLE fragrances (id INTEGER PRIMARY KEY, top_notes TEXT, heart_notes TEXT, base_notes TEXT);
            CREATE TABLE note_images (name TEXT PRIMARY KEY COLLATE NOCASE, image_data_url TEXT NOT NULL, parfumo_url TEXT NOT NULL DEFAULT '', categories TEXT NOT NULL DEFAULT '[]');
            INSERT INTO fragrances VALUES (1, '[\"Rose\",\"Apple\"]', '[\"rose\"]', '[\"Musk\"]');").unwrap();

        ensure_fragrance_notes_schema(&mut conn).unwrap();
        ensure_fragrance_notes_schema(&mut conn).unwrap();

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM fragrance_notes", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 4);
        let catalog_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM note_images", [], |row| row.get(0))
            .unwrap();
        assert_eq!(catalog_count, 3);
    }

    #[test]
    fn duplicate_fragrances_match_brand_and_name_but_allow_editing_the_same_row() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE fragrances (id INTEGER PRIMARY KEY, name TEXT, brand TEXT);
            INSERT INTO fragrances VALUES (1, 'Rose Garden', 'Maison Scent');",
        )
        .unwrap();
        let mut fragrance = Fragrance {
            id: None,
            collection: String::new(),
            name: " rose garden ".into(),
            brand: "MAISON SCENT".into(),
            year: 2024,
            audience: "Unisex".into(),
            concentration: String::new(),
            description: String::new(),
            top_notes: vec![],
            heart_notes: vec![],
            base_notes: vec![],
            perfumer: String::new(),
            bottle_rating: None,
            ratings: Ratings::default(),
            rating_count: None,
            image_url: String::new(),
            shelf: default_shelf(),
            bottles_owned: 0,
            bottles_finished: 0,
            favourite: false,
            price_bought: None,
            current_price: None,
        };

        assert_eq!(
            ensure_unique_fragrance(&conn, &fragrance, None).unwrap_err(),
            "This fragrance is already in your collection"
        );
        assert!(ensure_unique_fragrance(&conn, &fragrance, Some(1)).is_ok());
        fragrance.brand = "Another house".into();
        assert!(ensure_unique_fragrance(&conn, &fragrance, None).is_ok());
    }

    #[test]
    fn duplicate_notes_are_case_insensitive_and_renames_exclude_the_original() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE note_images (name TEXT PRIMARY KEY COLLATE NOCASE);
            INSERT INTO note_images VALUES ('Rose');
            INSERT INTO note_images VALUES ('Jasmine');",
        )
        .unwrap();

        assert_eq!(
            ensure_unique_note(&conn, " rose ", None).unwrap_err(),
            "That note already exists"
        );
        assert_eq!(
            ensure_unique_note(&conn, "JASMINE", Some("Rose")).unwrap_err(),
            "That note already exists"
        );
        assert!(ensure_unique_note(&conn, "rose", Some("Rose")).is_ok());
    }
}
