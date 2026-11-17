use super::*;
use csv::{ReaderBuilder, StringRecord, WriterBuilder};

const FRAGRANCE_HEADERS: [&str; 21] = [
    "collection",
    "name",
    "brand",
    "year",
    "audience",
    "description",
    "top_notes",
    "heart_notes",
    "base_notes",
    "perfumer",
    "bottle_rating",
    "rating_count",
    "image_url",
    "shelf",
    "bottles_owned",
    "bottles_finished",
    "favourite",
    "price_bought",
    "current_price",
    "concentration",
    "ratings",
];
const NOTE_HEADERS: [&str; 4] = ["name", "image_url", "parfumo_url", "categories"];

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ImportReport {
    imported: usize,
    duplicates: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct SyncReport {
    fragrances_added: usize,
    fragrances_updated: usize,
    notes_added: usize,
    notes_updated: usize,
}

#[derive(Serialize, Deserialize)]
struct FragranceRow {
    collection: String,
    name: String,
    brand: String,
    year: i32,
    audience: String,
    description: String,
    top_notes: String,
    heart_notes: String,
    base_notes: String,
    perfumer: String,
    bottle_rating: Option<f64>,
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
    #[serde(default)]
    concentration: String,
    #[serde(default)]
    ratings: String,
}

#[derive(Serialize, Deserialize)]
struct NoteRow {
    name: String,
    image_url: String,
    #[serde(default)]
    parfumo_url: String,
    #[serde(default)]
    categories: String,
}

fn notes_from_cell(value: &str) -> Result<Vec<String>, String> {
    if value.trim().is_empty() {
        return Ok(Vec::new());
    }
    serde_json::from_str(value)
        .map_err(|_| "Note columns must contain JSON arrays, such as [\"Rose\",\"Musk\"]".into())
}

impl TryFrom<FragranceRow> for Fragrance {
    type Error = String;

    fn try_from(row: FragranceRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: None,
            collection: row.collection,
            name: row.name,
            brand: row.brand,
            year: row.year,
            audience: row.audience,
            concentration: row.concentration,
            description: row.description,
            top_notes: notes_from_cell(&row.top_notes)?,
            heart_notes: notes_from_cell(&row.heart_notes)?,
            base_notes: notes_from_cell(&row.base_notes)?,
            perfumer: row.perfumer,
            bottle_rating: row.bottle_rating,
            ratings: if row.ratings.is_empty() { Ratings::default() } else { serde_json::from_str(&row.ratings).map_err(|e| e.to_string())? },
            rating_count: row.rating_count,
            image_url: row.image_url,
            shelf: row.shelf,
            bottles_owned: row.bottles_owned,
            bottles_finished: row.bottles_finished,
            favourite: row.favourite,
            price_bought: row.price_bought,
            current_price: row.current_price,
        })
    }
}

fn validate_headers(headers: &StringRecord, expected: &[&str]) -> Result<(), String> {
    let actual: HashSet<_> = headers.iter().collect();
    if headers.len() != expected.len() || !expected.iter().all(|header| actual.contains(header)) {
        return Err(format!(
            "Unsupported CSV columns. Expected exactly: {}",
            expected.join(", ")
        ));
    }
    Ok(())
}

fn reader(content: &str) -> csv::Reader<&[u8]> {
    ReaderBuilder::new().from_reader(content.trim_start_matches('\u{feff}').as_bytes())
}

fn csv_string(writer: csv::Writer<Vec<u8>>) -> Result<String, String> {
    String::from_utf8(writer.into_inner().map_err(|e| e.to_string())?).map_err(|e| e.to_string())
}

pub(super) fn fragrance_csv(conn: &Connection) -> Result<String, String> {
    let mut writer = WriterBuilder::new()
        .has_headers(false)
        .from_writer(Vec::new());
    writer
        .write_record(FRAGRANCE_HEADERS)
        .map_err(|e| e.to_string())?;
    for fragrance in load_fragrances(conn)? {
        writer
            .serialize(FragranceRow {
                collection: fragrance.collection,
                name: fragrance.name,
                brand: fragrance.brand,
                year: fragrance.year,
                audience: fragrance.audience,
                concentration: fragrance.concentration,
                description: fragrance.description,
                top_notes: serde_json::to_string(&fragrance.top_notes)
                    .map_err(|e| e.to_string())?,
                heart_notes: serde_json::to_string(&fragrance.heart_notes)
                    .map_err(|e| e.to_string())?,
                base_notes: serde_json::to_string(&fragrance.base_notes)
                    .map_err(|e| e.to_string())?,
                perfumer: fragrance.perfumer,
                bottle_rating: fragrance.ratings.overall().or(fragrance.bottle_rating),
                rating_count: fragrance.rating_count,
                image_url: fragrance.image_url,
                shelf: fragrance.shelf,
                bottles_owned: fragrance.bottles_owned,
                bottles_finished: fragrance.bottles_finished,
                favourite: fragrance.favourite,
                price_bought: fragrance.price_bought,
                current_price: fragrance.current_price,
                ratings: serde_json::to_string(&fragrance.ratings).map_err(|e| e.to_string())?,
            })
            .map_err(|e| e.to_string())?;
    }
    csv_string(writer)
}

fn parse_fragrances(content: &str) -> Result<Vec<(usize, Fragrance)>, String> {
    let mut reader = reader(content);
    let headers = reader
        .headers()
        .map_err(|e| format!("Invalid CSV header: {e}"))?
        .clone();
    if headers.len() == 13 {
        validate_headers(&headers, &FRAGRANCE_HEADERS[..13])?;
    } else if headers.len() == 19 {
        validate_headers(&headers, &FRAGRANCE_HEADERS[..19])?;
    } else if headers.len() == 20 {
        validate_headers(&headers, &FRAGRANCE_HEADERS[..20])?;
    } else {
        validate_headers(&headers, &FRAGRANCE_HEADERS)?;
    }
    let mut incoming = Vec::new();
    for (index, row) in reader.deserialize::<FragranceRow>().enumerate() {
        let row = row.map_err(|e| format!("CSV row {} is invalid: {e}", index + 2))?;
        let fragrance =
            Fragrance::try_from(row).map_err(|e| format!("CSV row {}: {e}", index + 2))?;
        validate_fragrance(&fragrance).map_err(|e| format!("CSV row {}: {e}", index + 2))?;
        incoming.push((index + 2, fragrance));
    }
    Ok(incoming)
}

fn import_fragrances(conn: &mut Connection, content: &str) -> Result<ImportReport, String> {
    let incoming = parse_fragrances(content)?;
    let transaction = conn
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .map_err(|e| e.to_string())?;
    let mut existing = HashSet::new();
    {
        let mut statement = transaction
            .prepare("SELECT name, brand FROM fragrances")
            .map_err(|e| e.to_string())?;
        let rows = statement
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(|e| e.to_string())?;
        for row in rows {
            let (name, brand) = row.map_err(|e| e.to_string())?;
            existing.insert((name.trim().to_lowercase(), brand.trim().to_lowercase()));
        }
    }
    let mut report = ImportReport {
        imported: 0,
        duplicates: Vec::new(),
    };
    for (row, fragrance) in incoming {
        let key = (
            fragrance.name.trim().to_lowercase(),
            fragrance.brand.trim().to_lowercase(),
        );
        if !existing.insert(key) {
            report.duplicates.push(format!(
                "{} — {} (row {row})",
                fragrance.brand.trim(),
                fragrance.name.trim()
            ));
            continue;
        }
        insert_fragrance(&transaction, &fragrance)?;
        report.imported += 1;
    }
    transaction.commit().map_err(|e| e.to_string())?;
    Ok(report)
}

pub(super) fn note_csv(conn: &Connection) -> Result<String, String> {
    let mut writer = WriterBuilder::new()
        .has_headers(false)
        .from_writer(Vec::new());
    writer
        .write_record(NOTE_HEADERS)
        .map_err(|e| e.to_string())?;
    for note in load_note_images(conn)? {
        writer
            .serialize(NoteRow {
                name: note.name,
                image_url: note.image_url,
                parfumo_url: note.parfumo_url,
                categories: serde_json::to_string(&note.categories).map_err(|e| e.to_string())?,
            })
            .map_err(|e| e.to_string())?;
    }
    csv_string(writer)
}

fn parse_notes(content: &str) -> Result<Vec<(usize, NoteRow)>, String> {
    let mut reader = reader(content);
    let headers = reader
        .headers()
        .map_err(|e| format!("Invalid CSV header: {e}"))?
        .clone();
    let legacy = (headers.len() == 2
        && headers.iter().collect::<HashSet<_>>() == ["name", "image_url"].into_iter().collect())
        || (headers.len() == 3
            && headers.iter().collect::<HashSet<_>>()
                == ["name", "image_url", "parfumo_url"].into_iter().collect());
    if !legacy {
        validate_headers(&headers, &NOTE_HEADERS)?;
    }
    let mut incoming = Vec::new();
    for (index, row) in reader.deserialize::<NoteRow>().enumerate() {
        let row = row.map_err(|e| format!("CSV row {} is invalid: {e}", index + 2))?;
        if row.name.trim().is_empty() {
            return Err(format!("CSV row {}: Enter a note name", index + 2));
        }
        if !row.image_url.trim().is_empty() {
            validate_image_url(&row.image_url)
                .map_err(|e| format!("CSV row {}: {e}", index + 2))?;
        }
        if !row.parfumo_url.trim().is_empty() && parfumo::parfumo_link(&row.parfumo_url).is_empty()
        {
            return Err(format!("CSV row {}: Enter a valid Parfumo link", index + 2));
        }
        if !row.categories.trim().is_empty()
            && serde_json::from_str::<Vec<String>>(&row.categories).is_err()
        {
            return Err(format!(
                "CSV row {}: Categories must be a JSON array",
                index + 2
            ));
        }
        incoming.push((index + 2, row));
    }
    Ok(incoming)
}

fn import_notes(conn: &mut Connection, content: &str) -> Result<ImportReport, String> {
    let incoming = parse_notes(content)?;
    let transaction = conn
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .map_err(|e| e.to_string())?;
    let mut existing = HashSet::new();
    {
        let mut statement = transaction
            .prepare("SELECT name FROM note_images")
            .map_err(|e| e.to_string())?;
        let rows = statement
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|e| e.to_string())?;
        for row in rows {
            existing.insert(row.map_err(|e| e.to_string())?.trim().to_lowercase());
        }
    }
    let mut report = ImportReport {
        imported: 0,
        duplicates: Vec::new(),
    };
    for (row, note) in incoming {
        if !existing.insert(note.name.trim().to_lowercase()) {
            report
                .duplicates
                .push(format!("{} (row {row})", note.name.trim()));
            continue;
        }
        transaction
            .execute(
                "INSERT INTO note_images (name, image_data_url, parfumo_url, categories) VALUES (?1, ?2, ?3, ?4)",
                params![note.name.trim(), note.image_url.trim(), note.parfumo_url.trim(), if note.categories.trim().is_empty() { "[]" } else { note.categories.trim() }],
            )
            .map_err(|e| e.to_string())?;
        report.imported += 1;
    }
    transaction.commit().map_err(|e| e.to_string())?;
    Ok(report)
}

pub(super) fn sync_backup(
    conn: &mut Connection,
    fragrances_csv: &str,
    notes_csv: &str,
) -> Result<SyncReport, String> {
    // Validate both files before changing either table.
    let fragrances = parse_fragrances(fragrances_csv)?;
    let notes = parse_notes(notes_csv)?;
    let transaction = conn
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .map_err(|e| e.to_string())?;
    let mut fragrance_ids = HashMap::new();
    for saved in load_fragrances(&transaction)? {
        fragrance_ids.insert(
            (
                saved.name.trim().to_lowercase(),
                saved.brand.trim().to_lowercase(),
            ),
            saved.id.unwrap(),
        );
    }
    let mut note_names = HashSet::new();
    {
        let mut statement = transaction
            .prepare("SELECT name FROM note_images")
            .map_err(|e| e.to_string())?;
        let rows = statement
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|e| e.to_string())?;
        for row in rows {
            note_names.insert(row.map_err(|e| e.to_string())?.trim().to_lowercase());
        }
    }
    let mut report = SyncReport {
        fragrances_added: 0,
        fragrances_updated: 0,
        notes_added: 0,
        notes_updated: 0,
    };
    for (_, fragrance) in fragrances {
        let key = (
            fragrance.name.trim().to_lowercase(),
            fragrance.brand.trim().to_lowercase(),
        );
        if let Some(id) = fragrance_ids.get(&key) {
            transaction.execute("UPDATE fragrances SET collection = ?1, name = ?2, brand = ?3, year = ?4, audience = ?5, description = ?6, top_notes = ?7, heart_notes = ?8, base_notes = ?9, perfumer = ?10, bottle_rating = ?11, rating_count = ?12, image_url = ?13, shelf = ?14, bottles_owned = ?15, bottles_finished = ?16, favourite = ?17, price_bought = ?18, current_price = ?19, concentration = ?20, ratings = ?22 WHERE id = ?21", params![
                fragrance.collection.trim(), fragrance.name.trim(), fragrance.brand.trim(), fragrance.year,
                fragrance.audience, fragrance.description.trim(),
                serde_json::to_string(&fragrance.top_notes).map_err(|e| e.to_string())?,
                serde_json::to_string(&fragrance.heart_notes).map_err(|e| e.to_string())?,
                serde_json::to_string(&fragrance.base_notes).map_err(|e| e.to_string())?,
                fragrance.perfumer.trim(), fragrance.ratings.overall().or(fragrance.bottle_rating), fragrance.rating_count, fragrance.image_url.trim(),
                fragrance.shelf, fragrance.bottles_owned, fragrance.bottles_finished, fragrance.favourite,
                fragrance.price_bought, fragrance.current_price, fragrance.concentration.trim(), id, serde_json::to_string(&fragrance.ratings).map_err(|e| e.to_string())?
            ]).map_err(|e| e.to_string())?;
            report.fragrances_updated += 1;
        } else {
            insert_fragrance(&transaction, &fragrance)?;
            fragrance_ids.insert(key, transaction.last_insert_rowid());
            report.fragrances_added += 1;
        }
    }
    for (_, note) in notes {
        if note_names.insert(note.name.trim().to_lowercase()) {
            transaction.execute("INSERT INTO note_images (name, image_data_url, parfumo_url, categories) VALUES (?1, ?2, ?3, ?4)",
                params![note.name.trim(), note.image_url.trim(), note.parfumo_url.trim(), if note.categories.trim().is_empty() { "[]" } else { note.categories.trim() }]).map_err(|e| e.to_string())?;
            report.notes_added += 1;
        } else {
            transaction.execute("UPDATE note_images SET image_data_url = ?2, parfumo_url = ?3, categories = ?4 WHERE name = ?1 COLLATE NOCASE",
                params![note.name.trim(), note.image_url.trim(), note.parfumo_url.trim(), if note.categories.trim().is_empty() { "[]" } else { note.categories.trim() }]).map_err(|e| e.to_string())?;
            report.notes_updated += 1;
        }
    }
    transaction.commit().map_err(|e| e.to_string())?;
    Ok(report)
}

#[tauri::command]
pub(super) fn export_fragrances_csv(app: tauri::AppHandle, path: String) -> Result<(), String> {
    if !path.to_lowercase().ends_with(".csv") {
        return Err("Choose a .csv filename".into());
    }
    let content = fragrance_csv(&database(&app)?)?;
    std::fs::write(path, content).map_err(|e| e.to_string())
}

#[tauri::command]
pub(super) fn import_fragrances_csv(
    app: tauri::AppHandle,
    content: String,
) -> Result<ImportReport, String> {
    let report = import_fragrances(&mut database(&app)?, &content)?;
    if report.imported > 0 {
        backup::after_change(&app)?;
    }
    Ok(report)
}

#[tauri::command]
pub(super) fn export_notes_csv(app: tauri::AppHandle, path: String) -> Result<(), String> {
    if !path.to_lowercase().ends_with(".csv") {
        return Err("Choose a .csv filename".into());
    }
    let content = note_csv(&database(&app)?)?;
    std::fs::write(path, content).map_err(|e| e.to_string())
}

#[tauri::command]
pub(super) fn import_notes_csv(
    app: tauri::AppHandle,
    content: String,
) -> Result<ImportReport, String> {
    let report = import_notes(&mut database(&app)?, &content)?;
    if report.imported > 0 {
        backup::after_change(&app)?;
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE fragrances (
            id INTEGER PRIMARY KEY, collection TEXT, name TEXT, brand TEXT, year INTEGER,
            audience TEXT, description TEXT, top_notes TEXT, heart_notes TEXT, base_notes TEXT,
            perfumer TEXT, bottle_rating REAL, ratings TEXT NOT NULL DEFAULT '{}', rating_count INTEGER, image_url TEXT,
            shelf TEXT NOT NULL DEFAULT 'Collection', bottles_owned INTEGER NOT NULL DEFAULT 0,
            bottles_finished INTEGER NOT NULL DEFAULT 0, favourite INTEGER NOT NULL DEFAULT 0,
            price_bought REAL, current_price REAL, concentration TEXT NOT NULL DEFAULT ''
        ); CREATE TABLE note_images (name TEXT PRIMARY KEY COLLATE NOCASE, image_data_url TEXT NOT NULL, parfumo_url TEXT NOT NULL DEFAULT '', categories TEXT NOT NULL DEFAULT '[]');").unwrap();
        conn
    }

    #[test]
    fn fragrance_csv_round_trip_and_duplicate_report() {
        let mut conn = test_db();
        let content = concat!(
            "collection,name,brand,year,audience,description,top_notes,heart_notes,base_notes,perfumer,bottle_rating,rating_count,image_url,shelf,bottles_owned,bottles_finished,favourite,price_bought,current_price\n",
            "Fresh,Rose Garden,Maison,2024,Unisex,Floral,\"[\"\"Rose\"\"]\",[],[],Alex,8.5,2,,Collection,1,0,false,50,\n",
            "Fresh, rose garden ,MAISON,2024,Unisex,Floral,[],[],[],Alex,,,,Collection,0,0,false,,\n"
        );
        let report = import_fragrances(&mut conn, content).unwrap();
        assert_eq!(report.imported, 1);
        assert_eq!(report.duplicates.len(), 1);
        assert!(report.duplicates[0].contains("row 3"));
        let repeated = import_fragrances(&mut conn, content).unwrap();
        assert_eq!(repeated.imported, 0);
        assert_eq!(repeated.duplicates.len(), 2);
        let exported = fragrance_csv(&conn).unwrap();
        let mut next = test_db();
        assert_eq!(import_fragrances(&mut next, &exported).unwrap().imported, 1);
        assert_eq!(load_fragrances(&next).unwrap()[0].top_notes, vec!["Rose"]);
    }

    #[test]
    fn unsupported_structure_and_bad_rows_do_not_write_anything() {
        let mut conn = test_db();
        assert!(
            import_fragrances(&mut conn, "name,brand\nRose,Maison\n")
                .unwrap_err()
                .contains("Unsupported CSV columns")
        );
        let content = concat!(
            "collection,name,brand,year,audience,description,top_notes,heart_notes,base_notes,perfumer,bottle_rating,rating_count,image_url,shelf,bottles_owned,bottles_finished,favourite,price_bought,current_price\n",
            "Fresh,Rose,Maison,2024,Unisex,,[],[],[],,,,,Collection,0,0,false,,\n",
            "Fresh,Jasmine,Maison,invalid,Unisex,,[],[],[],,,,,Collection,0,0,false,,\n"
        );
        assert!(
            import_fragrances(&mut conn, content)
                .unwrap_err()
                .contains("row 3")
        );
        assert!(load_fragrances(&conn).unwrap().is_empty());
    }

    #[test]
    fn note_csv_round_trip_and_duplicate_report() {
        let mut conn = test_db();
        let report = import_notes(
            &mut conn,
            "name,image_url\nRose,https://example.com/rose.png\n rose ,\n",
        )
        .unwrap();
        assert_eq!(report.imported, 1);
        assert_eq!(report.duplicates.len(), 1);
        let repeated = import_notes(&mut conn, "name,image_url\nROSE,\n").unwrap();
        assert_eq!(repeated.imported, 0);
        assert_eq!(repeated.duplicates.len(), 1);
        let exported = note_csv(&conn).unwrap();
        let mut next = test_db();
        assert_eq!(import_notes(&mut next, &exported).unwrap().imported, 1);
        assert!(exported.contains("https://example.com/rose.png"));
    }

    #[test]
    fn webdav_sync_merges_both_files_and_rejects_partial_data() {
        let mut source = test_db();
        let first = concat!(
            "collection,name,brand,year,audience,description,top_notes,heart_notes,base_notes,perfumer,bottle_rating,rating_count,image_url,shelf,bottles_owned,bottles_finished,favourite,price_bought,current_price\n",
            "Fresh,Rose,Maison,2024,Unisex,Original,[],[],[],,,,,Collection,0,0,false,,\n"
        );
        import_fragrances(&mut source, first).unwrap();
        import_notes(
            &mut source,
            "name,image_url\nRose,https://example.com/old.png\n",
        )
        .unwrap();
        let mut target = test_db();
        sync_backup(
            &mut target,
            &fragrance_csv(&source).unwrap(),
            &note_csv(&source).unwrap(),
        )
        .unwrap();
        source
            .execute(
                "UPDATE fragrances SET description = 'Updated' WHERE name = 'Rose'",
                [],
            )
            .unwrap();
        source.execute("UPDATE note_images SET image_data_url = 'https://example.com/new.png' WHERE name = 'Rose'", []).unwrap();
        let report = sync_backup(
            &mut target,
            &fragrance_csv(&source).unwrap(),
            &note_csv(&source).unwrap(),
        )
        .unwrap();
        assert_eq!(report.fragrances_updated, 1);
        assert_eq!(report.notes_updated, 1);
        assert_eq!(load_fragrances(&target).unwrap()[0].description, "Updated");
        assert!(note_csv(&target).unwrap().contains("new.png"));
        assert!(
            sync_backup(
                &mut target,
                &fragrance_csv(&source).unwrap(),
                "name,image_url\nRose,not-a-url\n"
            )
            .is_err()
        );
        assert_eq!(load_fragrances(&target).unwrap()[0].description, "Updated");
    }
}
