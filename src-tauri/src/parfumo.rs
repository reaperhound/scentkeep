use serde::{Deserialize, Serialize};
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};
use url::Url;
use scraper::{Html, Selector};
use std::time::Duration;
#[cfg(target_os = "linux")]
use std::sync::atomic::{AtomicU64, Ordering};
#[cfg(target_os = "linux")]
use webkit2gtk::WebViewExt;
#[cfg(target_os = "linux")]
use javascriptcore::ValueExt;

const LOGIN_WINDOW: &str = "parfumo-login";
const KEYRING_SERVICE: &str = "com.scentkeep.app.parfumo";
const KEYRING_USER: &str = "session";
const PARFUMO_URL: &str = "https://www.parfumo.com/";
#[cfg(target_os = "linux")]
static NOTE_SEARCH_ID: AtomicU64 = AtomicU64::new(0);
#[cfg(not(target_os = "linux"))]
const NOTE_SEARCH_URL: &str = "https://www.parfumo.com/action/livesearch/livesearch_note.php";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ParfumoNote {
    name: String,
    #[serde(default)]
    image_url: String,
    #[serde(default)]
    parfumo_url: String,
}

pub(crate) fn parfumo_link(value: &str) -> String {
    if value.trim().is_empty() { return String::new(); }
    Url::parse(PARFUMO_URL).ok()
        .and_then(|base| base.join(value.trim()).ok())
        .filter(|url| url.scheme() == "https" && matches!(url.host_str(), Some("www.parfumo.com" | "parfumo.com")))
        .map(|url| url.to_string()).unwrap_or_default()
}

fn image_link(value: &str) -> String {
    if value.trim().is_empty() { return String::new(); }
    Url::parse(PARFUMO_URL).ok()
        .and_then(|base| base.join(value.trim()).ok())
        .filter(|url| matches!(url.scheme(), "https" | "http") && url.host_str().is_some())
        .map(|mut url| {
            if url.host_str() == Some("media.parfumo.com") && url.path().starts_with("/notes_png/") {
                let retained = url.query_pairs()
                    .filter(|(key, _)| key != "width" && key != "aspect_ratio")
                    .map(|(key, value)| (key.into_owned(), value.into_owned()))
                    .collect::<Vec<_>>();
                url.set_query(None);
                if !retained.is_empty() {
                    let mut query = url.query_pairs_mut();
                    for (key, value) in retained { query.append_pair(&key, &value); }
                }
            }
            url.to_string()
        }).unwrap_or_default()
}

pub(crate) fn upgrade_note_images(conn: &rusqlite::Connection) -> Result<usize, String> {
    let mut statement = conn.prepare("SELECT name, image_data_url FROM note_images WHERE image_data_url LIKE 'https://media.parfumo.com/notes_png/%?%'")
        .map_err(|e| e.to_string())?;
    let rows = statement.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))
        .map_err(|e| e.to_string())?;
    let mut updates = Vec::new();
    for row in rows {
        let (name, old_url) = row.map_err(|e| e.to_string())?;
        let new_url = image_link(&old_url);
        if new_url != old_url { updates.push((name, new_url)); }
    }
    drop(statement);
    for (name, url) in &updates {
        conn.execute("UPDATE note_images SET image_data_url = ?2 WHERE name = ?1", rusqlite::params![name, url])
            .map_err(|e| e.to_string())?;
    }
    Ok(updates.len())
}

fn json_note(value: &serde_json::Value) -> Option<ParfumoNote> {
    let object = value.as_object()?;
    let name = ["name", "note", "title", "label", "text", "value", "note_name"]
        .iter().find_map(|key| object.get(*key).and_then(|v| v.as_str()))?.trim().to_owned();
    if name.is_empty() { return None; }
    let image_url = ["image", "image_url", "imageUrl", "img", "photo", "src"]
        .iter().find_map(|key| object.get(*key).and_then(|v| v.as_str())).map(image_link).unwrap_or_default();
    let parfumo_url = ["url", "link", "href"]
        .iter().find_map(|key| object.get(*key).and_then(|v| v.as_str())).map(parfumo_link).unwrap_or_default();
    Some(ParfumoNote { name, image_url, parfumo_url })
}

fn parse_notes(body: &str) -> Vec<ParfumoNote> {
    let mut notes = Vec::new();
    let mut html_body = body.to_owned();
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(body) {
        let items = value.as_array().or_else(|| value.get("results").and_then(|v| v.as_array()))
            .or_else(|| value.get("data").and_then(|v| v.as_array()))
            .or_else(|| value.get("items").and_then(|v| v.as_array()));
        if let Some(items) = items {
            notes.extend(items.iter().filter_map(json_note));
        }
        if let Some(markup) = value.as_str().or_else(|| value.get("html").and_then(|v| v.as_str())) {
            html_body = markup.to_owned();
        }
    }
    if notes.is_empty() {
        let html = Html::parse_fragment(&html_body);
        let link = Selector::parse("[data-name_url], a[href], [data-note-name], [data-name]").expect("valid selector");
        let image = Selector::parse("img").expect("valid selector");
        for item in html.select(&link) {
            let href = item.value().attr("href").unwrap_or_default();
            let name = item.text().collect::<Vec<_>>().join(" ").split_whitespace().collect::<Vec<_>>().join(" ");
            let name = item.value().attr("data-note-name").or_else(|| item.value().attr("data-name"))
                .map(str::to_owned).unwrap_or_else(|| if name.is_empty() { item.value().attr("title").unwrap_or_default().trim().to_owned() } else { name });
            if name.is_empty() { continue; }
            let image_url = item.select(&image).next().and_then(|img| img.value().attr("src").or_else(|| img.value().attr("data-src"))).map(image_link).unwrap_or_default();
            let parfumo_url = item.value().attr("data-name_url")
                .map(|slug| parfumo_link(&format!("/Fragrance_Note/{slug}")))
                .unwrap_or_else(|| parfumo_link(href));
            notes.push(ParfumoNote { name, image_url, parfumo_url });
        }
    }
    let mut seen = std::collections::HashSet::new();
    notes.retain(|note| seen.insert(note.name.to_lowercase()));
    notes.truncate(20);
    notes
}

#[tauri::command]
pub(crate) async fn search_parfumo_notes(app: tauri::AppHandle, query: String) -> Result<Vec<ParfumoNote>, String> {
    let query = query.trim().to_owned();
    if query.chars().count() < 2 { return Ok(Vec::new()); }
    if query.chars().count() > 100 { return Err("Search term is too long".into()); }
    if load_session()?.is_none() { return Err("Connect Parfumo in Settings before searching its notes.".into()); }
    #[cfg(target_os = "linux")]
    return search_notes_in_webview(app, query).await;
    #[cfg(not(target_os = "linux"))]
    return search_notes_with_session(query).await;
}

#[cfg(target_os = "linux")]
fn evaluate_note_script(window: &tauri::WebviewWindow, script: &str) -> Result<String, String> {
    let (sender, receiver) = std::sync::mpsc::channel();
    let script = script.to_owned();
    window.with_webview(move |webview| {
        webview.inner().evaluate_javascript(&script, None, None, None::<&webkit2gtk::gio::Cancellable>, move |result| {
            let response = result.map(|value| value.to_str().to_string()).map_err(|error| error.to_string());
            let _ = sender.send(response);
        });
    }).map_err(|error| format!("Could not search in Parfumo browser: {error}"))?;
    receiver.recv_timeout(Duration::from_secs(2))
        .map_err(|_| "Parfumo browser did not respond to the search.".to_string())?
}

#[cfg(target_os = "linux")]
async fn search_notes_in_webview(app: tauri::AppHandle, query: String) -> Result<Vec<ParfumoNote>, String> {
    use tauri::webview::PageLoadEvent;
    let mut ready = None;
    let window = if let Some(window) = app.get_webview_window(LOGIN_WINDOW) {
        window
    } else {
        let (sender, receiver) = std::sync::mpsc::channel();
        let url = Url::parse("https://www.parfumo.com/Fragrance_Notes").map_err(|e| e.to_string())?;
        let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?.join("parfumo-webview");
        let window = WebviewWindowBuilder::new(&app, LOGIN_WINDOW, WebviewUrl::External(url))
            .title("Parfumo notes")
            .inner_size(1100.0, 760.0)
            .visible(false)
            .data_directory(data_dir)
            .on_page_load(move |_, payload| {
                if payload.event() == PageLoadEvent::Finished { let _ = sender.send(()); }
            })
            .build().map_err(|e| format!("Could not open Parfumo browser: {e}"))?;
        ready = Some(receiver);
        window
    };
    if let Some(receiver) = ready {
        tauri::async_runtime::spawn_blocking(move || receiver.recv_timeout(Duration::from_secs(15)))
            .await.map_err(|e| e.to_string())?
            .map_err(|_| "Parfumo browser did not finish loading. Try again.".to_string())?;
    }
    if !matches!(window.url().ok().and_then(|url| url.host_str().map(str::to_owned)).as_deref(), Some("www.parfumo.com" | "parfumo.com")) {
        return Err("Open Parfumo in Settings and return to its website before searching notes.".into());
    }
    let query_js = serde_json::to_string(&query).map_err(|e| e.to_string())?;
    let search_id = NOTE_SEARCH_ID.fetch_add(1, Ordering::Relaxed);
    let script = format!(r#"(() => {{
        const responses = window.__scentkeepNoteSearchResponses ??= new Map();
        const searchId = {search_id};
        (async () => {{
          try {{
            const controller = new AbortController();
            const timeout = setTimeout(() => controller.abort(), 12000);
            try {{
                const response = await fetch('/action/livesearch/livesearch_note.php', {{
                    method: 'POST',
                    headers: {{ 'Content-Type': 'application/x-www-form-urlencoded;charset=UTF-8' }},
                    body: new URLSearchParams({{ q: {query_js}, csrf_key: '' }}),
                    credentials: 'same-origin',
                    signal: controller.signal
                }});
                responses.set(searchId, JSON.stringify({{ status: response.status, body: await response.text() }}));
            }} finally {{ clearTimeout(timeout); }}
          }} catch (error) {{ responses.set(searchId, JSON.stringify({{ error: String(error) }})); }}
          finally {{ setTimeout(() => responses.delete(searchId), 30000); }}
        }})();
        return 'started';
    }})()"#);
    let response = tauri::async_runtime::spawn_blocking(move || {
        evaluate_note_script(&window, &script)?;
        let poll_script = format!("(() => {{ const responses = window.__scentkeepNoteSearchResponses; const result = responses?.get({search_id}); if (result) responses.delete({search_id}); return result ?? ''; }})()");
        let deadline = std::time::Instant::now() + Duration::from_secs(15);
        loop {
            let response = evaluate_note_script(&window, &poll_script)?;
            if !response.is_empty() { break Ok(response); }
            if std::time::Instant::now() >= deadline { break Err("Parfumo search timed out. Try again.".to_string()); }
            std::thread::sleep(Duration::from_millis(100));
        }
    }).await.map_err(|error| error.to_string())??;
    let value: serde_json::Value = serde_json::from_str(&response).map_err(|_| "Parfumo returned an invalid search response".to_string())?;
    if let Some(error) = value.get("error").and_then(|value| value.as_str()) { return Err(format!("Parfumo browser search failed: {error}")); }
    let status = value.get("status").and_then(|value| value.as_u64()).unwrap_or(0);
    if status == 403 {
        if let Some(window) = app.get_webview_window(LOGIN_WINDOW) {
            let _ = window.show();
            let _ = window.set_focus();
        }
        return Err("Parfumo opened for a browser check. Complete it there, then try the note search again.".into());
    }
    if status != 200 { return Err(format!("Parfumo search returned HTTP {status}.")); }
    let body = value.get("body").and_then(|value| value.as_str()).unwrap_or_default();
    Ok(parse_notes(body))
}

#[cfg(not(target_os = "linux"))]
async fn search_notes_with_session(query: String) -> Result<Vec<ParfumoNote>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let session = load_session()?.ok_or("Connect Parfumo in Settings before searching its notes.")?;
        let cookies = session.cookies.iter()
            .filter(|cookie| cookie.domain.as_deref().is_none_or(|domain| domain.trim_start_matches('.').ends_with("parfumo.com")))
            .map(|cookie| format!("{}={}", cookie.name, cookie.value)).collect::<Vec<_>>().join("; ");
        let client = reqwest::blocking::Client::builder().timeout(Duration::from_secs(12)).build().map_err(|e| e.to_string())?;
        let response = client.post(NOTE_SEARCH_URL)
            .header(reqwest::header::COOKIE, cookies)
            .header(reqwest::header::REFERER, "https://www.parfumo.com/Fragrance_Notes")
            .header(reqwest::header::USER_AGENT, "Mozilla/5.0 (X11; Linux x86_64; rv:159.0) Gecko/20100101 Firefox/159.0")
            .form(&[("q", query.as_str()), ("csrf_key", "")])
            .send().map_err(|e| format!("Parfumo search failed: {e}"))?;
        if !response.status().is_success() { return Err(format!("Parfumo search returned HTTP {}. Refresh your session in Settings.", response.status())); }
        let body = response.text().map_err(|e| e.to_string())?;
        if body.contains("Just a moment...") && body.contains("challenges.cloudflare.com") {
            return Err("Parfumo requires a fresh browser check. Open Parfumo in Settings, then save the session again.".into());
        }
        Ok(parse_notes(&body))
    }).await.map_err(|e| e.to_string())?
}

#[tauri::command]
pub(crate) fn save_parfumo_note(app: tauri::AppHandle, note: ParfumoNote) -> Result<(), String> {
    let name = note.name.trim();
    if name.is_empty() || name.chars().count() > 200 { return Err("Choose a valid note".into()); }
    let image_url = if note.image_url.is_empty() { String::new() } else { image_link(&note.image_url) };
    let parfumo_url = if note.parfumo_url.is_empty() { String::new() } else { parfumo_link(&note.parfumo_url) };
    let mut conn = crate::database(&app)?;
    let transaction = conn.transaction().map_err(|e| e.to_string())?;
    transaction.execute("INSERT INTO note_images (name, image_data_url, parfumo_url) VALUES (?1, ?2, ?3) ON CONFLICT(name) DO UPDATE SET image_data_url = CASE WHEN excluded.image_data_url = '' THEN note_images.image_data_url ELSE excluded.image_data_url END, parfumo_url = CASE WHEN excluded.parfumo_url = '' THEN note_images.parfumo_url ELSE excluded.parfumo_url END", rusqlite::params![name, image_url, parfumo_url]).map_err(|e| e.to_string())?;
    transaction.commit().map_err(|e| e.to_string())?;
    crate::backup::after_change(&app)
}

#[cfg(test)]
mod note_search_tests {
    use super::*;

    #[test]
    fn parses_html_suggestions_and_keeps_parfumo_links() {
        let notes = parse_notes(r#"<a href="/Fragrance_Notes/Caramel"><img src="/img/caramel.jpg">Caramel</a><a href="https://evil.example/note">Other</a>"#);
        assert_eq!(notes[0].name, "Caramel");
        assert_eq!(notes[0].image_url, "https://www.parfumo.com/img/caramel.jpg");
        assert_eq!(notes[0].parfumo_url, "https://www.parfumo.com/Fragrance_Notes/Caramel");
        assert!(notes[1].parfumo_url.is_empty());
    }

    #[test]
    fn parses_json_suggestions() {
        let notes = parse_notes(r#"{"results":[{"name":"Cardamom","image":"/img/cardamom.jpg","url":"/Fragrance_Notes/Cardamom"}]}"#);
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].name, "Cardamom");
    }

    #[test]
    fn parses_parfumo_note_table_rows() {
        let notes = parse_notes(r#"<table id="ls_popup_results_notes"><tr><td><div class="pad pointer bold" data-name_url="Oud"><img src="https://media.parfumo.com/notes_png/oud.png" alt="Oud"> Oud</div></td></tr><tr><td><div data-name_url="Cambodian_oud"><img src="https://media.parfumo.com/notes_png/oud.png" alt="Cambodian oud"> Cambodian oud</div></td></tr></table>"#);
        assert_eq!(notes.len(), 2);
        assert_eq!(notes[0].name, "Oud");
        assert_eq!(notes[0].parfumo_url, "https://www.parfumo.com/Fragrance_Note/Oud");
        assert_eq!(notes[1].name, "Cambodian oud");
    }

    #[test]
    fn uses_full_resolution_note_images_and_upgrades_saved_thumbnails() {
        let thumbnail = "https://media.parfumo.com/notes_png/9d/oud_1200.png?width=40&aspect_ratio=1:1";
        let full = "https://media.parfumo.com/notes_png/9d/oud_1200.png";
        assert_eq!(image_link(thumbnail), full);
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE note_images (name TEXT PRIMARY KEY, image_data_url TEXT NOT NULL);").unwrap();
        conn.execute("INSERT INTO note_images VALUES ('Oud', ?1)", [thumbnail]).unwrap();
        assert_eq!(upgrade_note_images(&conn).unwrap(), 1);
        let saved: String = conn.query_row("SELECT image_data_url FROM note_images WHERE name = 'Oud'", [], |row| row.get(0)).unwrap();
        assert_eq!(saved, full);
        assert_eq!(upgrade_note_images(&conn).unwrap(), 0);
    }
}

// Keep the session on the Rust side. Future importers can use this record to
// build requests without exposing cookie values to the Svelte webview.
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct SavedCookie {
    pub name: String,
    pub value: String,
    pub domain: Option<String>,
    pub path: Option<String>,
    pub secure: Option<bool>,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct SavedSession {
    pub cookies: Vec<SavedCookie>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ParfumoStatus {
    connected: bool,
    cookie_count: usize,
}

fn keyring_entry() -> Result<keyring::Entry, String> {
    keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER).map_err(|e| e.to_string())
}

pub(crate) fn load_session() -> Result<Option<SavedSession>, String> {
    let secret = match keyring_entry()?.get_password() {
        Ok(secret) => secret,
        Err(keyring::Error::NoEntry) => return Ok(None),
        Err(e) => return Err(format!("Could not read the system keyring: {e}")),
    };
    serde_json::from_str(&secret)
        .map(Some)
        .map_err(|_| "The saved Parfumo session is invalid. Disconnect and reconnect it.".into())
}

#[tauri::command]
pub(crate) fn parfumo_status() -> Result<ParfumoStatus, String> {
    let session = load_session()?;
    Ok(ParfumoStatus {
        connected: session.is_some(),
        cookie_count: session.map_or(0, |session| session.cookies.len()),
    })
}

#[tauri::command]
pub(crate) fn open_parfumo_login(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window(LOGIN_WINDOW) {
        window.show().map_err(|e| e.to_string())?;
        return window.set_focus().map_err(|e| e.to_string());
    }

    let url = Url::parse(PARFUMO_URL).map_err(|e| e.to_string())?;
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("parfumo-webview");
    WebviewWindowBuilder::new(&app, LOGIN_WINDOW, WebviewUrl::External(url))
        .title("Connect Parfumo")
        .inner_size(1100.0, 760.0)
        .data_directory(data_dir)
        .build()
        .map_err(|e| format!("Could not open Parfumo: {e}"))?;
    Ok(())
}

#[tauri::command]
pub(crate) async fn save_parfumo_session(app: tauri::AppHandle) -> Result<ParfumoStatus, String> {
    let window = app
        .get_webview_window(LOGIN_WINDOW)
        .ok_or("Open the Parfumo login window and sign in first.")?;
    tauri::async_runtime::spawn_blocking(move || {
        let url = Url::parse(PARFUMO_URL).map_err(|e| e.to_string())?;
        let cookies = window
            .cookies_for_url(url)
            .map_err(|e| format!("Could not read the Parfumo browser session: {e}"))?
            .into_iter()
            .map(|cookie| SavedCookie {
                name: cookie.name().to_owned(),
                value: cookie.value().to_owned(),
                domain: cookie.domain().map(str::to_owned),
                path: cookie.path().map(str::to_owned),
                secure: cookie.secure(),
            })
            .collect::<Vec<_>>();

        if cookies.is_empty() {
            return Err(
                "No Parfumo cookies were found. Sign in in the Parfumo window, then try again."
                    .into(),
            );
        }

        let cookie_count = cookies.len();
        let session =
            serde_json::to_string(&SavedSession { cookies }).map_err(|e| e.to_string())?;
        keyring_entry()?
            .set_password(&session)
            .map_err(|e| format!("Could not save the session to the system keyring: {e}"))?;
        Ok(ParfumoStatus {
            connected: true,
            cookie_count,
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub(crate) async fn disconnect_parfumo(app: tauri::AppHandle) -> Result<(), String> {
    let window = app.get_webview_window(LOGIN_WINDOW);
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("parfumo-webview");
    tauri::async_runtime::spawn_blocking(move || {
        let entry = keyring_entry()?;
        match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => {}
            Err(e) => return Err(format!("Could not remove the saved session: {e}")),
        }
        if let Some(window) = window {
            window
                .clear_all_browsing_data()
                .map_err(|e| e.to_string())?;
            window.close().map_err(|e| e.to_string())?;
        } else if data_dir.exists() {
            std::fs::remove_dir_all(&data_dir)
                .map_err(|e| format!("Could not clear Parfumo browser data: {e}"))?;
        }
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}
