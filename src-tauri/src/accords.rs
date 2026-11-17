use rusqlite::{Connection, OptionalExtension};
use serde::Serialize;
use std::collections::{HashMap, HashSet};

const MAX_ACCORDS: usize = 7;
const TOP_NOTE_WEIGHT: f64 = 0.9;
const HEART_NOTE_WEIGHT: f64 = 1.0;
const BASE_NOTE_WEIGHT: f64 = 1.05;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AccordBar {
    key: String,
    label: String,
    score: f64,
    relative_strength: u8,
    contributing_notes: Vec<String>,
}

struct AccordScore {
    label: String,
    score: f64,
    contributing_notes: Vec<String>,
}

#[tauri::command]
pub(crate) fn get_fragrance_accords(
    app: tauri::AppHandle,
    fragrance_id: i64,
) -> Result<Vec<AccordBar>, String> {
    let conn = crate::database(&app)?;
    calculate_fragrance_accords(&conn, fragrance_id)
}

fn calculate_fragrance_accords(
    conn: &Connection,
    fragrance_id: i64,
) -> Result<Vec<AccordBar>, String> {
    let layers = conn
        .query_row(
            "SELECT top_notes, heart_notes, base_notes FROM fragrances WHERE id = ?1",
            [fragrance_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )
        .optional()
        .map_err(|error| error.to_string())?
        .ok_or("Fragrance not found")?;

    let top_notes = parse_notes(&layers.0)?;
    let heart_notes = parse_notes(&layers.1)?;
    let base_notes = parse_notes(&layers.2)?;
    let category_map = load_note_categories(conn)?;
    let mut scores = HashMap::<String, AccordScore>::new();

    add_layer(&mut scores, &category_map, &top_notes, TOP_NOTE_WEIGHT);
    add_layer(&mut scores, &category_map, &heart_notes, HEART_NOTE_WEIGHT);
    add_layer(&mut scores, &category_map, &base_notes, BASE_NOTE_WEIGHT);

    let mut ranked = scores.into_iter().collect::<Vec<_>>();
    ranked.sort_by(|left, right| {
        right
            .1
            .score
            .total_cmp(&left.1.score)
            .then_with(|| left.1.label.cmp(&right.1.label))
    });
    ranked.truncate(MAX_ACCORDS);

    let strongest = ranked
        .first()
        .map(|(_, accord)| accord.score)
        .unwrap_or(0.0);
    Ok(ranked
        .into_iter()
        .map(|(key, accord)| AccordBar {
            key,
            label: accord.label,
            score: (accord.score * 100.0).round() / 100.0,
            relative_strength: if strongest > 0.0 {
                ((accord.score / strongest) * 100.0).round() as u8
            } else {
                0
            },
            contributing_notes: accord.contributing_notes,
        })
        .collect())
}

fn parse_notes(value: &str) -> Result<Vec<String>, String> {
    serde_json::from_str(value).map_err(|error| format!("Could not read fragrance notes: {error}"))
}

fn load_note_categories(conn: &Connection) -> Result<HashMap<String, Vec<String>>, String> {
    let mut statement = conn
        .prepare("SELECT name, categories FROM note_images")
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|error| error.to_string())?;
    let mut categories = HashMap::new();
    for row in rows {
        let (name, raw_categories) = row.map_err(|error| error.to_string())?;
        categories.insert(
            name.trim().to_lowercase(),
            serde_json::from_str(&raw_categories).unwrap_or_default(),
        );
    }
    Ok(categories)
}

fn add_layer(
    scores: &mut HashMap<String, AccordScore>,
    category_map: &HashMap<String, Vec<String>>,
    notes: &[String],
    weight: f64,
) {
    for note in notes {
        let note_name = note.trim();
        if note_name.is_empty() {
            continue;
        }
        let Some(categories) = category_map.get(&note_name.to_lowercase()) else {
            continue;
        };
        let mut categories_seen_for_note = HashSet::new();
        for category in categories {
            let Some((key, label)) = normalize_category(category) else {
                continue;
            };
            if !categories_seen_for_note.insert(key.clone()) {
                continue;
            }
            let accord = scores.entry(key).or_insert_with(|| AccordScore {
                label,
                score: 0.0,
                contributing_notes: Vec::new(),
            });
            accord.score += weight;
            if !accord
                .contributing_notes
                .iter()
                .any(|saved| saved.eq_ignore_ascii_case(note_name))
            {
                accord.contributing_notes.push(note_name.to_owned());
            }
        }
    }
}

fn normalize_category(value: &str) -> Option<(String, String)> {
    let replaced = value.trim().to_lowercase().replace(['_', '-'], " ");
    let mut key = replaced.split_whitespace().collect::<Vec<_>>().join(" ");
    if let Some(without_suffix) = key.strip_suffix(" notes") {
        key = without_suffix.to_owned();
    } else if let Some(without_suffix) = key.strip_suffix(" note") {
        key = without_suffix.to_owned();
    }
    key = match key.as_str() {
        "wood" | "woods" => "woody",
        "citrus fruits" | "citruses" => "citrus",
        "flowers" | "florals" => "floral",
        "fruits" => "fruity",
        "herbs" => "herbal",
        "musks" => "musky",
        "spices" => "spicy",
        other => other,
    }
    .to_owned();
    if key.is_empty() {
        return None;
    }
    let label = title_case(&key);
    Some((key, label))
}

fn title_case(value: &str) -> String {
    value
        .split_whitespace()
        .map(|word| {
            let mut characters = word.chars();
            match characters.next() {
                Some(first) => first.to_uppercase().collect::<String>() + characters.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
