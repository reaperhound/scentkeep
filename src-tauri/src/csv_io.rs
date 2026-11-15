use super::*;
use csv::{ReaderBuilder, StringRecord, WriterBuilder};

const FRAGRANCE_HEADERS: [&str; 20] = [
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
];
const NOTE_HEADERS: [&str; 2] = ["name", "image_url"];

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ImportReport {
    imported: usize,
    duplicates: Vec<String>,
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
}

#[derive(Serialize, Deserialize)]
struct NoteRow {
    name: String,
    image_url: String,
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
            rating_count: row.rating_count,
            image_url: row.image_url,
            shelf: row.shelf, bottles_owned: row.bottles_owned, bottles_finished: row.bottles_finished, favourite: row.favourite, price_bought: row.price_bought, current_price: row.current_price,
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

fn fragrance_csv(conn: &Connection) -> Result<String, String> {
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
                bottle_rating: fragrance.bottle_rating,
                rating_count: fragrance.rating_count,
                image_url: fragrance.image_url,
                shelf: fragrance.shelf, bottles_owned: fragrance.bottles_owned, bottles_finished: fragrance.bottles_finished, favourite: fragrance.favourite, price_bought: fragrance.price_bought, current_price: fragrance.current_price,
            })
            .map_err(|e| e.to_string())?;
    }
    csv_string(writer)
}

fn import_fragrances(conn: &mut Connection, content: &str) -> Result<ImportReport, String> {
    let mut reader = reader(content);
    let headers = reader
        .headers()
        .map_err(|e| format!("Invalid CSV header: {e}"))?
        .clone();
    if headers.len() == 13 { validate_headers(&headers, &FRAGRANCE_HEADERS[..13])?; }
    else if headers.len() == 19 { validate_headers(&headers, &FRAGRANCE_HEADERS[..19])?; }
    else { validate_headers(&headers, &FRAGRANCE_HEADERS)?; }
    let mut incoming = Vec::new();
    for (index, row) in reader.deserialize::<FragranceRow>().enumerate() {
        let row = row.map_err(|e| format!("CSV row {} is invalid: {e}", index + 2))?;
        let fragrance =
            Fragrance::try_from(row).map_err(|e| format!("CSV row {}: {e}", index + 2))?;
        validate_fragrance(&fragrance).map_err(|e| format!("CSV row {}: {e}", index + 2))?;
        incoming.push((index + 2, fragrance));
    }
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

fn note_csv(conn: &Connection) -> Result<String, String> {
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
            })
            .map_err(|e| e.to_string())?;
    }
    csv_string(writer)
}

fn import_notes(conn: &mut Connection, content: &str) -> Result<ImportReport, String> {
    let mut reader = reader(content);
    let headers = reader
        .headers()
        .map_err(|e| format!("Invalid CSV header: {e}"))?
        .clone();
    validate_headers(&headers, &NOTE_HEADERS)?;
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
        incoming.push((index + 2, row));
    }
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
                "INSERT INTO note_images (name, image_data_url) VALUES (?1, ?2)",
                params![note.name.trim(), note.image_url.trim()],
            )
            .map_err(|e| e.to_string())?;
        report.imported += 1;
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
    import_fragrances(&mut database(&app)?, &content)
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
    import_notes(&mut database(&app)?, &content)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE fragrances (
            id INTEGER PRIMARY KEY, collection TEXT, name TEXT, brand TEXT, year INTEGER,
            audience TEXT, description TEXT, top_notes TEXT, heart_notes TEXT, base_notes TEXT,
            perfumer TEXT, bottle_rating REAL, rating_count INTEGER, image_url TEXT,
            shelf TEXT NOT NULL DEFAULT 'Collection', bottles_owned INTEGER NOT NULL DEFAULT 0,
            bottles_finished INTEGER NOT NULL DEFAULT 0, favourite INTEGER NOT NULL DEFAULT 0,
            price_bought REAL, current_price REAL, concentration TEXT NOT NULL DEFAULT ''
        ); CREATE TABLE note_images (name TEXT PRIMARY KEY COLLATE NOCASE, image_data_url TEXT NOT NULL);").unwrap();
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
}
