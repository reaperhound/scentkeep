#[cfg(target_os = "linux")]
use javascriptcore::ValueExt;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
#[cfg(target_os = "linux")]
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};
use url::Url;
#[cfg(target_os = "linux")]
use webkit2gtk::{
    CookieAcceptPolicy, CookieManagerExt, SettingsExt, WebViewExt, WebsiteDataManagerExt,
    WebsiteDataManagerExtManual, WebsiteDataTypes,
};

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
    #[serde(default)]
    categories: Vec<String>,
}

pub(crate) fn parfumo_link(value: &str) -> String {
    if value.trim().is_empty() {
        return String::new();
    }
    Url::parse(PARFUMO_URL)
        .ok()
        .and_then(|base| base.join(value.trim()).ok())
        .filter(|url| {
            url.scheme() == "https"
                && matches!(url.host_str(), Some("www.parfumo.com" | "parfumo.com"))
        })
        .map(|url| url.to_string())
        .unwrap_or_default()
}

fn image_link(value: &str) -> String {
    if value.trim().is_empty() {
        return String::new();
    }
    Url::parse(PARFUMO_URL)
        .ok()
        .and_then(|base| base.join(value.trim()).ok())
        .filter(|url| matches!(url.scheme(), "https" | "http") && url.host_str().is_some())
        .map(|mut url| {
            if url.host_str() == Some("media.parfumo.com") && url.path().starts_with("/notes_png/")
            {
                let retained = url
                    .query_pairs()
                    .filter(|(key, _)| key != "width" && key != "aspect_ratio")
                    .map(|(key, value)| (key.into_owned(), value.into_owned()))
                    .collect::<Vec<_>>();
                url.set_query(None);
                if !retained.is_empty() {
                    let mut query = url.query_pairs_mut();
                    for (key, value) in retained {
                        query.append_pair(&key, &value);
                    }
                }
            }
            url.to_string()
        })
        .unwrap_or_default()
}

pub(crate) fn upgrade_note_images(conn: &rusqlite::Connection) -> Result<usize, String> {
    let mut statement = conn.prepare("SELECT name, image_data_url FROM note_images WHERE image_data_url LIKE 'https://media.parfumo.com/notes_png/%?%'")
        .map_err(|e| e.to_string())?;
    let rows = statement
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|e| e.to_string())?;
    let mut updates = Vec::new();
    for row in rows {
        let (name, old_url) = row.map_err(|e| e.to_string())?;
        let new_url = image_link(&old_url);
        if new_url != old_url {
            updates.push((name, new_url));
        }
    }
    drop(statement);
    for (name, url) in &updates {
        conn.execute(
            "UPDATE note_images SET image_data_url = ?2 WHERE name = ?1",
            rusqlite::params![name, url],
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(updates.len())
}

fn json_note(value: &serde_json::Value) -> Option<ParfumoNote> {
    let object = value.as_object()?;
    let name = [
        "name",
        "note",
        "title",
        "label",
        "text",
        "value",
        "note_name",
    ]
    .iter()
    .find_map(|key| object.get(*key).and_then(|v| v.as_str()))?
    .trim()
    .to_owned();
    if name.is_empty() {
        return None;
    }
    let image_url = ["image", "image_url", "imageUrl", "img", "photo", "src"]
        .iter()
        .find_map(|key| object.get(*key).and_then(|v| v.as_str()))
        .map(image_link)
        .unwrap_or_default();
    let parfumo_url = ["url", "link", "href"]
        .iter()
        .find_map(|key| object.get(*key).and_then(|v| v.as_str()))
        .map(parfumo_link)
        .unwrap_or_default();
    let categories = object
        .get("categories")
        .and_then(|value| value.as_array())
        .map(|values| {
            values
                .iter()
                .filter_map(|value| value.as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default();
    Some(ParfumoNote {
        name,
        image_url,
        parfumo_url,
        categories,
    })
}

fn parse_notes(body: &str) -> Vec<ParfumoNote> {
    let mut notes = Vec::new();
    let mut html_body = body.to_owned();
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(body) {
        let items = value
            .as_array()
            .or_else(|| value.get("results").and_then(|v| v.as_array()))
            .or_else(|| value.get("data").and_then(|v| v.as_array()))
            .or_else(|| value.get("items").and_then(|v| v.as_array()));
        if let Some(items) = items {
            notes.extend(items.iter().filter_map(json_note));
        }
        if let Some(markup) = value
            .as_str()
            .or_else(|| value.get("html").and_then(|v| v.as_str()))
        {
            html_body = markup.to_owned();
        }
    }
    if notes.is_empty() {
        let html = Html::parse_fragment(&html_body);
        let link = Selector::parse("[data-name_url], a[href], [data-note-name], [data-name]")
            .expect("valid selector");
        let image = Selector::parse("img").expect("valid selector");
        for item in html.select(&link) {
            let href = item.value().attr("href").unwrap_or_default();
            let name = item
                .text()
                .collect::<Vec<_>>()
                .join(" ")
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ");
            let name = item
                .value()
                .attr("data-note-name")
                .or_else(|| item.value().attr("data-name"))
                .map(str::to_owned)
                .unwrap_or_else(|| {
                    if name.is_empty() {
                        item.value()
                            .attr("title")
                            .unwrap_or_default()
                            .trim()
                            .to_owned()
                    } else {
                        name
                    }
                });
            if name.is_empty() {
                continue;
            }
            let image_url = item
                .select(&image)
                .next()
                .and_then(|img| {
                    img.value()
                        .attr("src")
                        .or_else(|| img.value().attr("data-src"))
                })
                .map(image_link)
                .unwrap_or_default();
            let parfumo_url = item
                .value()
                .attr("data-name_url")
                .map(|slug| parfumo_link(&format!("/Fragrance_Note/{slug}")))
                .unwrap_or_else(|| parfumo_link(href));
            notes.push(ParfumoNote {
                name,
                image_url,
                parfumo_url,
                categories: Vec::new(),
            });
        }
    }
    let mut seen = std::collections::HashSet::new();
    notes.retain(|note| seen.insert(note.name.to_lowercase()));
    notes.truncate(20);
    notes
}

#[tauri::command]
pub(crate) async fn search_parfumo_notes(
    app: tauri::AppHandle,
    query: String,
) -> Result<Vec<ParfumoNote>, String> {
    let query = query.trim().to_owned();
    if query.chars().count() < 2 {
        return Ok(Vec::new());
    }
    if query.chars().count() > 100 {
        return Err("Search term is too long".into());
    }
    if load_session()?.is_none() {
        return Err("Connect Parfumo in Settings before searching its notes.".into());
    }
    #[cfg(target_os = "linux")]
    return search_notes_in_webview(app, query).await;
    #[cfg(not(target_os = "linux"))]
    return search_notes_with_session(query).await;
}

#[cfg(target_os = "linux")]
fn configure_parfumo_webview(window: &tauri::WebviewWindow) -> Result<(), String> {
    let (sender, receiver) = std::sync::mpsc::channel();
    window
        .with_webview(move |webview| {
            let result = (|| {
                let settings = webview
                    .inner()
                    .settings()
                    .ok_or("Parfumo browser settings are unavailable")?;
                settings.set_enable_javascript(true);
                settings.set_enable_html5_local_storage(true);
                settings.set_enable_html5_database(true);
                let manager = webview
                    .inner()
                    .website_data_manager()
                    .ok_or("Parfumo browser storage is unavailable")?;
                let cookies = manager
                    .cookie_manager()
                    .ok_or("Parfumo browser cookies are unavailable")?;
                cookies.set_accept_policy(CookieAcceptPolicy::Always);
                Ok::<(), &str>(())
            })();
            let _ = sender.send(result);
        })
        .map_err(|error| format!("Could not configure Parfumo browser: {error}"))?;
    receiver
        .recv_timeout(Duration::from_secs(2))
        .map_err(|_| "Parfumo browser did not apply its settings.".to_string())?
        .map_err(str::to_owned)
}

#[cfg(target_os = "linux")]
fn evaluate_browser_script(window: &tauri::WebviewWindow, script: &str) -> Result<String, String> {
    let (sender, receiver) = std::sync::mpsc::channel();
    let script = script.to_owned();
    window
        .with_webview(move |webview| {
            webview.inner().evaluate_javascript(
                &script,
                None,
                None,
                None::<&webkit2gtk::gio::Cancellable>,
                move |result| {
                    let response = result
                        .map(|value| value.to_str().to_string())
                        .map_err(|error| error.to_string());
                    let _ = sender.send(response);
                },
            );
        })
        .map_err(|error| format!("Could not read Parfumo browser: {error}"))?;
    receiver
        .recv_timeout(Duration::from_secs(2))
        .map_err(|_| "Parfumo browser did not respond.".to_string())?
}

#[cfg(target_os = "linux")]
async fn search_notes_in_webview(
    app: tauri::AppHandle,
    query: String,
) -> Result<Vec<ParfumoNote>, String> {
    let body = fetch_in_webview(
        app,
        "/action/livesearch/livesearch_note.php".into(),
        Some(query),
    )
    .await?;
    Ok(parse_notes(&body))
}

#[cfg(target_os = "linux")]
async fn fetch_in_webview(
    app: tauri::AppHandle,
    path: String,
    query: Option<String>,
) -> Result<String, String> {
    use tauri::webview::PageLoadEvent;
    let mut ready = None;
    let window = if let Some(window) = app.get_webview_window(LOGIN_WINDOW) {
        window
    } else {
        let (sender, receiver) = std::sync::mpsc::channel();
        let url =
            Url::parse("https://www.parfumo.com/Fragrance_Notes").map_err(|e| e.to_string())?;
        let data_dir = app
            .path()
            .app_data_dir()
            .map_err(|e| e.to_string())?
            .join("parfumo-webview");
        let window = WebviewWindowBuilder::new(&app, LOGIN_WINDOW, WebviewUrl::External(url))
            .title("Parfumo notes")
            .inner_size(1100.0, 760.0)
            .visible(false)
            .data_directory(data_dir)
            .on_page_load(move |_, payload| {
                if payload.event() == PageLoadEvent::Finished {
                    let _ = sender.send(());
                }
            })
            .build()
            .map_err(|e| format!("Could not open Parfumo browser: {e}"))?;
        ready = Some(receiver);
        window
    };
    configure_parfumo_webview(&window)?;
    if let Some(receiver) = ready {
        tauri::async_runtime::spawn_blocking(move || {
            receiver.recv_timeout(Duration::from_secs(15))
        })
        .await
        .map_err(|e| e.to_string())?
        .map_err(|_| "Parfumo browser did not finish loading. Try again.".to_string())?;
    }
    if !matches!(
        window
            .url()
            .ok()
            .and_then(|url| url.host_str().map(str::to_owned))
            .as_deref(),
        Some("www.parfumo.com" | "parfumo.com")
    ) {
        return Err(
            "Open Parfumo in Settings and return to its website before searching notes.".into(),
        );
    }
    let query_js = serde_json::to_string(&query).map_err(|e| e.to_string())?;
    let path_js = serde_json::to_string(&path).map_err(|e| e.to_string())?;
    let search_id = NOTE_SEARCH_ID.fetch_add(1, Ordering::Relaxed);
    let script = format!(
        r#"(() => {{
        const responses = window.__scentkeepNoteSearchResponses ??= new Map();
        const searchId = {search_id};
        (async () => {{
          try {{
            const controller = new AbortController();
            const timeout = setTimeout(() => controller.abort(), 12000);
            try {{
                const response = await fetch({path_js}, {{
                    method: {query_js} === null ? 'GET' : 'POST',
                    headers: {{ 'Content-Type': 'application/x-www-form-urlencoded;charset=UTF-8' }},
                    body: {query_js} === null ? undefined : new URLSearchParams({{ q: {query_js}, csrf_key: '' }}),
                    credentials: 'same-origin',
                    signal: controller.signal
                }});
                responses.set(searchId, JSON.stringify({{ status: response.status, body: await response.text() }}));
            }} finally {{ clearTimeout(timeout); }}
          }} catch (error) {{ responses.set(searchId, JSON.stringify({{ error: String(error) }})); }}
          finally {{ setTimeout(() => responses.delete(searchId), 30000); }}
        }})();
        return 'started';
    }})()"#
    );
    let response = tauri::async_runtime::spawn_blocking(move || {
        evaluate_browser_script(&window, &script)?;
        let poll_script = format!("(() => {{ const responses = window.__scentkeepNoteSearchResponses; const result = responses?.get({search_id}); if (result) responses.delete({search_id}); return result ?? ''; }})()");
        let deadline = std::time::Instant::now() + Duration::from_secs(15);
        loop {
            let response = evaluate_browser_script(&window, &poll_script)?;
            if !response.is_empty() { break Ok(response); }
            if std::time::Instant::now() >= deadline { break Err("Parfumo search timed out. Try again.".to_string()); }
            std::thread::sleep(Duration::from_millis(100));
        }
    }).await.map_err(|error| error.to_string())??;
    let value: serde_json::Value = serde_json::from_str(&response)
        .map_err(|_| "Parfumo returned an invalid search response".to_string())?;
    if let Some(error) = value.get("error").and_then(|value| value.as_str()) {
        return Err(format!("Parfumo browser search failed: {error}"));
    }
    let status = value
        .get("status")
        .and_then(|value| value.as_u64())
        .unwrap_or(0);
    if status == 403 {
        if let Some(window) = app.get_webview_window(LOGIN_WINDOW) {
            let _ = window.show();
            let _ = window.set_focus();
        }
        return Err(
            "Parfumo opened for a browser check. Complete it there, then try again.".into(),
        );
    }
    if status != 200 {
        return Err(format!("Parfumo search returned HTTP {status}."));
    }
    let body = value
        .get("body")
        .and_then(|value| value.as_str())
        .unwrap_or_default();
    check_fragrance_response(body)?;
    Ok(body.to_owned())
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

#[cfg_attr(target_os = "linux", allow(dead_code))]
fn categories_from_html(body: &str, note_name: &str) -> Vec<String> {
    let html = Html::parse_document(body);
    let links = Selector::parse("a[href]").expect("valid selector");
    let mut categories = Vec::new();
    for link in html.select(&links) {
        let href = link.value().attr("href").unwrap_or_default();
        if !href.contains("/Fragrance_Note/") && !href.contains("/Fragrance_Notes/") {
            continue;
        }
        let label = link
            .text()
            .collect::<Vec<_>>()
            .join(" ")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        if label.is_empty()
            || label.eq_ignore_ascii_case(note_name)
            || label.eq_ignore_ascii_case("Fragrance Notes")
        {
            continue;
        }
        if !categories
            .iter()
            .any(|saved: &String| saved.eq_ignore_ascii_case(&label))
        {
            categories.push(label);
        }
    }
    clean_categories(note_name, categories)
}

pub(crate) fn clean_categories(note_name: &str, categories: Vec<String>) -> Vec<String> {
    let mut cleaned = Vec::new();
    for category in categories {
        let label = category.split_whitespace().collect::<Vec<_>>().join(" ");
        let lower = label.to_lowercase();
        if label.is_empty()
            || label.eq_ignore_ascii_case(note_name)
            || matches!(
                lower.as_str(),
                "fragrance notes" | "scentmix" | "next" | "previous"
            )
            || label.chars().all(|character| character.is_ascii_digit())
            || !label.chars().any(char::is_alphabetic)
            || cleaned
                .iter()
                .any(|saved: &String| saved.eq_ignore_ascii_case(&label))
        {
            continue;
        }
        cleaned.push(label);
    }
    cleaned
}

pub(crate) fn clean_saved_note_categories(conn: &rusqlite::Connection) -> Result<usize, String> {
    let mut statement = conn
        .prepare("SELECT name, categories FROM note_images WHERE categories != '[]'")
        .map_err(|e| e.to_string())?;
    let rows = statement
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|e| e.to_string())?;
    let mut updates = Vec::new();
    for row in rows {
        let (name, raw) = row.map_err(|e| e.to_string())?;
        let cleaned = serde_json::to_string(&clean_categories(
            &name,
            serde_json::from_str(&raw).unwrap_or_default(),
        ))
        .map_err(|e| e.to_string())?;
        if cleaned != raw {
            updates.push((name, cleaned));
        }
    }
    drop(statement);
    for (name, categories) in &updates {
        conn.execute(
            "UPDATE note_images SET categories = ?2 WHERE name = ?1",
            rusqlite::params![name, categories],
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(updates.len())
}

#[cfg(target_os = "linux")]
fn categories_in_webview(
    window: &tauri::WebviewWindow,
    notes: &[(String, String)],
) -> Result<Vec<Vec<String>>, String> {
    let payload = serde_json::to_string(notes).map_err(|e| e.to_string())?;
    let search_id = NOTE_SEARCH_ID.fetch_add(1, Ordering::Relaxed);
    let script = format!(
        r#"(() => {{
      const responses = window.__scentkeepNoteSearchResponses ??= new Map();
      const items = {payload};
      const categoryLabels = (html, noteName) => {{
        const document = new DOMParser().parseFromString(html, 'text/html');
        const seen = new Set();
        return [...document.querySelectorAll('a[href*="/Fragrance_Note/"], a[href*="/Fragrance_Notes/"]')].slice(0, 12).map(link => link.textContent.trim().replace(/\s+/g, ' ')).filter(label => label && label.toLocaleLowerCase() !== noteName.toLocaleLowerCase() && label.toLocaleLowerCase() !== 'fragrance notes' && !seen.has(label.toLocaleLowerCase()) && seen.add(label.toLocaleLowerCase()));
      }};
      (async () => {{
        try {{
          const result = [];
          for (const [name, url] of items) {{
            const response = await fetch(url, {{ credentials: 'same-origin' }});
            result.push(response.ok ? categoryLabels(await response.text(), name) : []);
          }}
          responses.set({search_id}, JSON.stringify(result));
        }} catch (error) {{ responses.set({search_id}, JSON.stringify({{ error: String(error) }})); }}
      }})();
      return 'started';
    }})()"#
    );
    evaluate_browser_script(window, &script)?;
    let poll = format!(
        "(() => {{ const result = window.__scentkeepNoteSearchResponses?.get({search_id}); if (result) window.__scentkeepNoteSearchResponses.delete({search_id}); return result ?? ''; }})()"
    );
    let deadline = std::time::Instant::now() + Duration::from_secs(30);
    loop {
        let response = evaluate_browser_script(window, &poll)?;
        if !response.is_empty() {
            let value: serde_json::Value =
                serde_json::from_str(&response).map_err(|e| e.to_string())?;
            if let Some(error) = value.get("error").and_then(|value| value.as_str()) {
                return Err(error.to_owned());
            }
            let categories: Vec<Vec<String>> =
                serde_json::from_value(value).map_err(|e| e.to_string())?;
            return Ok(notes
                .iter()
                .zip(categories)
                .map(|((name, _), values)| clean_categories(name, values))
                .collect());
        }
        if std::time::Instant::now() >= deadline {
            return Err("Timed out reading Parfumo note categories".into());
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

#[cfg(not(target_os = "linux"))]
fn categories_with_session(notes: &[(String, String)]) -> Result<Vec<Vec<String>>, String> {
    let session = load_session()?.ok_or("Connect Parfumo in Settings first.")?;
    let cookies = session
        .cookies
        .iter()
        .filter(|cookie| {
            cookie
                .domain
                .as_deref()
                .is_none_or(|domain| domain.trim_start_matches('.').ends_with("parfumo.com"))
        })
        .map(|cookie| format!("{}={}", cookie.name, cookie.value))
        .collect::<Vec<_>>()
        .join("; ");
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(12))
        .build()
        .map_err(|e| e.to_string())?;
    notes
        .iter()
        .map(|(name, url)| {
            let response = client
                .get(url)
                .header(reqwest::header::COOKIE, &cookies)
                .header(reqwest::header::USER_AGENT, "Mozilla/5.0")
                .send()
                .map_err(|e| e.to_string())?;
            Ok(if response.status().is_success() {
                categories_from_html(&response.text().map_err(|e| e.to_string())?, name)
            } else {
                Vec::new()
            })
        })
        .collect()
}

#[tauri::command]
pub(crate) async fn save_parfumo_note(
    app: tauri::AppHandle,
    mut note: ParfumoNote,
) -> Result<crate::NoteImage, String> {
    if note.name.trim().is_empty() || note.name.trim().chars().count() > 200 {
        return Err("Choose a valid note".into());
    }
    let _ = populate_note_categories(&app, std::slice::from_mut(&mut note)).await;
    let mut conn = crate::database(&app)?;
    let transaction = conn.transaction().map_err(|e| e.to_string())?;
    store_parfumo_note(&transaction, &note, true)?;
    transaction.commit().map_err(|e| e.to_string())?;
    crate::backup::after_change(&app)?;
    crate::load_note_image(&conn, note.name.trim())
}

async fn populate_note_categories(
    app: &tauri::AppHandle,
    notes: &mut [ParfumoNote],
) -> Result<(), String> {
    let requests: Vec<_> = notes
        .iter()
        .filter_map(|note| {
            let url = parfumo_link(&note.parfumo_url);
            (note.categories.is_empty() && !url.is_empty()).then(|| (note.name.clone(), url))
        })
        .collect();
    if requests.is_empty() {
        return Ok(());
    }
    #[cfg(target_os = "linux")]
    let window = app
        .get_webview_window(LOGIN_WINDOW)
        .ok_or("Parfumo browser is unavailable")?;
    #[cfg(not(target_os = "linux"))]
    let _ = app;
    let found = tauri::async_runtime::spawn_blocking(move || {
        #[cfg(target_os = "linux")]
        let categories = categories_in_webview(&window, &requests)?;
        #[cfg(not(target_os = "linux"))]
        let categories = categories_with_session(&requests)?;
        Ok::<_, String>(requests.into_iter().zip(categories).collect::<Vec<_>>())
    })
    .await
    .map_err(|error| error.to_string())??;
    for ((name, _), categories) in found {
        if let Some(note) = notes.iter_mut().find(|note| note.name == name) {
            note.categories = clean_categories(&note.name, categories);
        }
    }
    Ok(())
}

fn store_parfumo_note(
    conn: &rusqlite::Connection,
    note: &ParfumoNote,
    overwrite_existing: bool,
) -> Result<(), String> {
    let name = note.name.trim();
    if name.is_empty() || name.chars().count() > 200 {
        return Err("Choose a valid note".into());
    }
    let image_url = image_link(&note.image_url);
    let parfumo_url = parfumo_link(&note.parfumo_url);
    if (!note.image_url.trim().is_empty() && image_url.is_empty())
        || (!note.parfumo_url.trim().is_empty() && parfumo_url.is_empty())
    {
        return Err(format!(
            "Invalid Parfumo image or source link for note {name}."
        ));
    }
    let categories = serde_json::to_string(&clean_categories(name, note.categories.clone()))
        .map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO note_images (name, image_data_url, parfumo_url, categories)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(name) DO UPDATE SET
           image_data_url = CASE WHEN excluded.image_data_url != '' AND (?5 OR note_images.image_data_url = '') THEN excluded.image_data_url ELSE note_images.image_data_url END,
           parfumo_url = CASE WHEN excluded.parfumo_url != '' AND (?5 OR note_images.parfumo_url = '') THEN excluded.parfumo_url ELSE note_images.parfumo_url END,
           categories = CASE WHEN excluded.categories != '[]' AND (?5 OR note_images.categories = '[]') THEN excluded.categories ELSE note_images.categories END",
        rusqlite::params![name, image_url, parfumo_url, categories, overwrite_existing],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

pub(crate) fn save_fragrance_notes(
    conn: &rusqlite::Connection,
    layers: [&[String]; 3],
    imported_notes: &[ParfumoNote],
) -> Result<(), String> {
    let names: std::collections::HashSet<_> = layers
        .into_iter()
        .flatten()
        .map(|name| name.trim().to_lowercase())
        .collect();
    for note in imported_notes {
        // A user can remove an imported note before saving the fragrance.
        if names.contains(&note.name.trim().to_lowercase()) {
            store_parfumo_note(conn, note, false)?;
        }
    }
    Ok(())
}

#[tauri::command]
pub(crate) async fn refresh_parfumo_note_metadata(app: tauri::AppHandle) -> Result<usize, String> {
    let notes = {
        let conn = crate::database(&app)?;
        let mut statement = conn
            .prepare("SELECT name, parfumo_url FROM note_images WHERE categories = '[]'")
            .map_err(|e| e.to_string())?;
        statement
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(|e| e.to_string())?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|e| e.to_string())?
    };
    if notes.is_empty() {
        return Ok(0);
    }
    let mut resolved = notes;
    #[cfg(target_os = "linux")]
    {
        if app.get_webview_window(LOGIN_WINDOW).is_none() {
            // Create the same hidden authenticated browser used by note search,
            // allowing old saved notes to be enriched after an app restart.
            let _ = search_notes_in_webview(app.clone(), "la".into()).await?;
        }
        for (name, url) in &mut resolved {
            if !url.is_empty() {
                continue;
            }
            if let Some(found) = search_notes_in_webview(app.clone(), name.clone())
                .await?
                .into_iter()
                .find(|note| note.name.eq_ignore_ascii_case(name))
            {
                *url = found.parfumo_url;
            }
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        for (name, url) in &mut resolved {
            if !url.is_empty() {
                continue;
            }
            if let Some(found) = search_notes_with_session(name.clone())
                .await?
                .into_iter()
                .find(|note| note.name.eq_ignore_ascii_case(name))
            {
                *url = found.parfumo_url;
            }
        }
    }
    resolved.retain(|(_, url)| !url.is_empty());
    if resolved.is_empty() {
        return Ok(0);
    }
    #[cfg(target_os = "linux")]
    let found = categories_in_webview(
        &app.get_webview_window(LOGIN_WINDOW)
            .ok_or("Parfumo browser is unavailable")?,
        &resolved,
    )?;
    #[cfg(not(target_os = "linux"))]
    let found = categories_with_session(&resolved)?;
    let conn = crate::database(&app)?;
    let mut updated = 0;
    for ((name, url), categories) in resolved.into_iter().zip(found) {
        if categories.is_empty() {
            continue;
        }
        conn.execute(
            "UPDATE note_images SET categories = ?2, parfumo_url = CASE WHEN parfumo_url = '' THEN ?3 ELSE parfumo_url END WHERE name = ?1",
            rusqlite::params![
                name,
                serde_json::to_string(&categories).map_err(|e| e.to_string())?, url
            ],
        )
        .map_err(|e| e.to_string())?;
        updated += 1;
    }
    if updated > 0 {
        crate::backup::after_change(&app)?;
    }
    Ok(updated)
}

#[cfg(test)]
mod note_search_tests {
    use super::*;

    #[test]
    fn challenge_cookies_are_not_a_usable_parfumo_session() {
        for name in ["cf_clearance", "__cf_bm", "_cfuvid"] {
            assert!(is_security_cookie(name));
        }
        assert!(!is_security_cookie("PHPSESSID"));
    }

    #[test]
    fn parses_html_suggestions_and_keeps_parfumo_links() {
        let notes = parse_notes(
            r#"<a href="/Fragrance_Notes/Caramel"><img src="/img/caramel.jpg">Caramel</a><a href="https://evil.example/note">Other</a>"#,
        );
        assert_eq!(notes[0].name, "Caramel");
        assert_eq!(
            notes[0].image_url,
            "https://www.parfumo.com/img/caramel.jpg"
        );
        assert_eq!(
            notes[0].parfumo_url,
            "https://www.parfumo.com/Fragrance_Notes/Caramel"
        );
        assert!(notes[1].parfumo_url.is_empty());
    }

    #[test]
    fn parses_json_suggestions() {
        let notes = parse_notes(
            r#"{"results":[{"name":"Cardamom","image":"/img/cardamom.jpg","url":"/Fragrance_Notes/Cardamom"}]}"#,
        );
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].name, "Cardamom");
    }

    #[test]
    fn parses_parfumo_note_table_rows() {
        let notes = parse_notes(
            r#"<table id="ls_popup_results_notes"><tr><td><div class="pad pointer bold" data-name_url="Oud"><img src="https://media.parfumo.com/notes_png/oud.png" alt="Oud"> Oud</div></td></tr><tr><td><div data-name_url="Cambodian_oud"><img src="https://media.parfumo.com/notes_png/oud.png" alt="Cambodian oud"> Cambodian oud</div></td></tr></table>"#,
        );
        assert_eq!(notes.len(), 2);
        assert_eq!(notes[0].name, "Oud");
        assert_eq!(
            notes[0].parfumo_url,
            "https://www.parfumo.com/Fragrance_Note/Oud"
        );
        assert_eq!(notes[1].name, "Cambodian oud");
    }

    #[test]
    fn parses_note_categories_from_detail_page_hierarchy() {
        let categories = categories_from_html(
            r#"<a href="/Fragrance_Notes/">Fragrance Notes</a><a href="/Fragrance_Note/Lavender">Lavender</a><a href="/Fragrance_Notes/Floral_Notes">Floral Notes</a><a href="/Fragrance_Notes/Flowers">Flowers</a><a href="/Fragrance_Notes/Herbaceous_Notes">Herbaceous Notes</a>"#,
            "Lavender",
        );
        assert_eq!(categories, ["Floral Notes", "Flowers", "Herbaceous Notes"]);
    }

    #[test]
    fn removes_scentmix_and_pagination_from_categories() {
        assert_eq!(
            clean_categories(
                "Kiwi",
                ["Fruits", "ScentMix", "1", "2", "20", "Next", "Kiwi"]
                    .into_iter()
                    .map(str::to_owned)
                    .collect()
            ),
            ["Fruits"]
        );
    }

    #[test]
    fn uses_full_resolution_note_images_and_upgrades_saved_thumbnails() {
        let thumbnail =
            "https://media.parfumo.com/notes_png/9d/oud_1200.png?width=40&aspect_ratio=1:1";
        let full = "https://media.parfumo.com/notes_png/9d/oud_1200.png";
        assert_eq!(image_link(thumbnail), full);
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE note_images (name TEXT PRIMARY KEY, image_data_url TEXT NOT NULL);",
        )
        .unwrap();
        conn.execute("INSERT INTO note_images VALUES ('Oud', ?1)", [thumbnail])
            .unwrap();
        assert_eq!(upgrade_note_images(&conn).unwrap(), 1);
        let saved: String = conn
            .query_row(
                "SELECT image_data_url FROM note_images WHERE name = 'Oud'",
                [],
                |row| row.get(0),
            )
            .unwrap();
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

fn is_security_cookie(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    name.starts_with("cf_") || name.starts_with("_cf") || name.starts_with("__cf")
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
        #[cfg(target_os = "linux")]
        configure_parfumo_webview(&window)?;
        window.show().map_err(|e| e.to_string())?;
        return window.set_focus().map_err(|e| e.to_string());
    }

    let url = Url::parse(PARFUMO_URL).map_err(|e| e.to_string())?;
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("parfumo-webview");
    let window = WebviewWindowBuilder::new(&app, LOGIN_WINDOW, WebviewUrl::External(url))
        .title("Connect Parfumo")
        .inner_size(1100.0, 760.0)
        .data_directory(data_dir)
        .build()
        .map_err(|e| format!("Could not open Parfumo: {e}"))?;
    #[cfg(target_os = "linux")]
    configure_parfumo_webview(&window)?;
    #[cfg(not(target_os = "linux"))]
    let _ = window;
    Ok(())
}

#[tauri::command]
pub(crate) async fn save_parfumo_session(app: tauri::AppHandle) -> Result<ParfumoStatus, String> {
    let window = app
        .get_webview_window(LOGIN_WINDOW)
        .ok_or("Open the Parfumo login window and sign in first.")?;
    tauri::async_runtime::spawn_blocking(move || {
        let browser_url = window.url().map_err(|error| error.to_string())?;
        if !matches!(browser_url.host_str(), Some("www.parfumo.com" | "parfumo.com")) {
            return Err("The Parfumo browser is not on Parfumo. Open Parfumo and sign in before saving the session.".into());
        }
        #[cfg(target_os = "linux")]
        {
            let page_state = evaluate_browser_script(&window, r#"(() => {
                const title = document.title.toLowerCase();
                const text = (document.body?.innerText ?? '').slice(0, 3000).toLowerCase();
                if (title.includes('just a moment') || text.includes('performing security verification') || text.includes('verify you are human') || document.querySelector('iframe[src*="challenges.cloudflare.com"]')) return 'challenge';
                return document.readyState === 'loading' ? 'loading' : 'ready';
            })()"#)?;
            if page_state == "challenge" {
                return Err("Parfumo is still showing Cloudflare verification. Try Reset browser data in Settings, then reopen Parfumo. If it remains stuck, try a different network or contact Parfumo with the Ray ID shown on the page.".into());
            }
            if page_state != "ready" {
                return Err("Wait for the Parfumo page to finish loading before saving the session.".into());
            }
        }
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

        if !cookies.iter().any(|cookie| !is_security_cookie(&cookie.name)) {
            return Err(
                "No usable Parfumo session cookies were found. Finish signing in on Parfumo, then try again."
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

fn clear_parfumo_browser(app: &tauri::AppHandle) -> Result<(), String> {
    let window = app.get_webview_window(LOGIN_WINDOW);
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("parfumo-webview");
    if let Some(window) = window {
        #[cfg(target_os = "linux")]
        {
            let (sender, receiver) = std::sync::mpsc::channel();
            window
                .with_webview(move |webview| {
                    if let Some(manager) = webview.inner().website_data_manager() {
                        manager.clear(
                            WebsiteDataTypes::ALL,
                            webkit2gtk::glib::TimeSpan::from_seconds(0),
                            None::<&webkit2gtk::gio::Cancellable>,
                            move |result| {
                                let _ = sender.send(result.map_err(|error| error.to_string()));
                            },
                        );
                    } else {
                        let _ =
                            sender.send(Err("Parfumo browser storage is unavailable".to_string()));
                    }
                })
                .map_err(|error| error.to_string())?;
            receiver
                .recv_timeout(Duration::from_secs(10))
                .map_err(|_| "Timed out clearing Parfumo browser data".to_string())??;
        }
        #[cfg(not(target_os = "linux"))]
        window
            .clear_all_browsing_data()
            .map_err(|error| error.to_string())?;
        window.close().map_err(|error| error.to_string())?;
    } else if data_dir.exists() {
        std::fs::remove_dir_all(&data_dir)
            .map_err(|error| format!("Could not clear Parfumo browser data: {error}"))?;
    }
    Ok(())
}

#[tauri::command]
pub(crate) async fn reset_parfumo_browser(app: tauri::AppHandle) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || clear_parfumo_browser(&app))
        .await
        .map_err(|error| error.to_string())?
}

#[tauri::command]
pub(crate) async fn disconnect_parfumo(app: tauri::AppHandle) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let entry = keyring_entry()?;
        match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => {}
            Err(e) => return Err(format!("Could not remove the saved session: {e}")),
        }
        clear_parfumo_browser(&app)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ParfumoFragranceResult {
    name: String,
    url: String,
    image_url: String,
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ParfumoFragranceDraft {
    name: String,
    brand: String,
    year: Option<i32>,
    audience: String,
    concentration: String,
    perfumer: String,
    description: String,
    image_url: String,
    top_notes: Vec<String>,
    heart_notes: Vec<String>,
    base_notes: Vec<String>,
    notes: Vec<ParfumoNote>,
    source_url: String,
    warnings: Vec<String>,
}

fn check_fragrance_response(body: &str) -> Result<(), String> {
    if body.contains("challenges.cloudflare.com") || body.contains("Just a moment...") {
        return Err("Parfumo requires a browser check. Open Parfumo in Settings, complete the check and save the session, then try again.".into());
    }
    Ok(())
}

fn fragrance_url(value: &str) -> Result<String, String> {
    let link = parfumo_link(value);
    let url = Url::parse(&link).map_err(|_| "Choose a valid Parfumo fragrance link.")?;
    let parts: Vec<_> = url.path().trim_matches('/').split('/').collect();
    if parts.len() != 3 || parts[0] != "Perfumes" || parts[1].is_empty() || parts[2].is_empty() {
        return Err("Choose a fragrance page on parfumo.com/Perfumes/Brand/Fragrance.".into());
    }
    // Always use the same origin as the embedded browser.
    Ok(format!("https://www.parfumo.com{}", url.path()))
}

async fn fetch_fragrance_page(app: tauri::AppHandle, url: String) -> Result<String, String> {
    if load_session()?.is_none() {
        return Err("Connect Parfumo in Settings before searching fragrances.".into());
    }
    #[cfg(target_os = "linux")]
    {
        let parsed = Url::parse(&url).map_err(|e| e.to_string())?;
        let path = format!(
            "{}{}",
            parsed.path(),
            parsed.query().map(|q| format!("?{q}")).unwrap_or_default()
        );
        fetch_in_webview(app, path, None).await
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = app;
        tauri::async_runtime::spawn_blocking(move || {
            let session = load_session()?.ok_or("Connect Parfumo in Settings first.")?;
            let cookies = session
                .cookies
                .iter()
                .filter(|cookie| {
                    cookie.domain.as_deref().is_none_or(|domain| {
                        let domain = domain.trim_start_matches('.');
                        domain == "parfumo.com" || domain.ends_with(".parfumo.com")
                    })
                })
                .map(|cookie| format!("{}={}", cookie.name, cookie.value))
                .collect::<Vec<_>>()
                .join("; ");
            let client = reqwest::blocking::Client::builder()
                .timeout(Duration::from_secs(15))
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .map_err(|e| e.to_string())?;
            let response = client
                .get(url)
                .header(reqwest::header::COOKIE, cookies)
                .send()
                .map_err(|e| format!("Could not reach Parfumo: {e}"))?;
            if !response.status().is_success() {
                return Err(format!(
                    "Parfumo returned HTTP {}. Refresh your session in Settings.",
                    response.status()
                ));
            }
            let body = response.text().map_err(|e| e.to_string())?;
            check_fragrance_response(&body)?;
            Ok(body)
        })
        .await
        .map_err(|e| e.to_string())?
    }
}

fn text_of(element: scraper::ElementRef<'_>) -> String {
    element
        .text()
        .collect::<Vec<_>>()
        .join(" ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn field(html: &Html, selector: &str) -> String {
    html.select(&Selector::parse(selector).expect("valid selector"))
        .find_map(|element| {
            let value = element
                .value()
                .attr("content")
                .map(str::to_owned)
                .unwrap_or_else(|| text_of(element));
            (!value.trim().is_empty()).then(|| value.trim().to_owned())
        })
        .unwrap_or_default()
}

fn parse_fragrance_results(body: &str) -> Result<Vec<ParfumoFragranceResult>, String> {
    check_fragrance_response(body)?;
    let html = Html::parse_document(body);
    let mut seen = std::collections::HashSet::new();
    let mut results = Vec::new();
    for link in html.select(&Selector::parse("a[href]").unwrap()) {
        let Ok(url) = fragrance_url(link.value().attr("href").unwrap_or_default()) else {
            continue;
        };
        let image = link.select(&Selector::parse("img").unwrap()).next();
        let name = text_of(link);
        let name = if name.is_empty() {
            image
                .and_then(|img| img.value().attr("alt"))
                .unwrap_or_default()
                .to_owned()
        } else {
            name
        };
        if name.is_empty() || !seen.insert(url.clone()) {
            continue;
        }
        let image_url = image
            .and_then(|img| {
                img.value()
                    .attr("data-src")
                    .or_else(|| img.value().attr("src"))
            })
            .map(image_link)
            .unwrap_or_default();
        results.push(ParfumoFragranceResult {
            name,
            url,
            image_url,
        });
        if results.len() == 30 {
            break;
        }
    }
    Ok(results)
}

fn fragrance_note(element: scraper::ElementRef<'_>) -> Option<ParfumoNote> {
    let href = element.value().attr("href");
    let parfumo_url = href.map(parfumo_link).unwrap_or_default();
    if href.is_some() && parfumo_url.is_empty() {
        return None;
    }
    let image_selector = Selector::parse("img").unwrap();
    let image = element.select(&image_selector).next().or_else(|| {
        // Some pyramids place the image next to the note link.
        href?;
        let parent = element.parent().and_then(scraper::ElementRef::wrap)?;
        let links = Selector::parse("a[href*='/Fragrance_Note/']").unwrap();
        (parent.select(&links).count() == 1)
            .then(|| parent.select(&image_selector).next())
            .flatten()
    });
    let mut name = text_of(element);
    if name.is_empty() {
        name = image
            .and_then(|image| image.value().attr("alt"))
            .or_else(|| element.value().attr("title"))
            .unwrap_or_default()
            .trim()
            .to_owned();
    }
    if name.is_empty() {
        return None;
    }
    let image_url = image
        .and_then(|image| {
            image
                .value()
                .attr("data-src")
                .or_else(|| image.value().attr("src"))
        })
        .map(image_link)
        .unwrap_or_default();
    Some(ParfumoNote {
        name,
        image_url,
        parfumo_url,
        categories: Vec::new(),
    })
}

fn apply_note_search_result(note: &mut ParfumoNote, results: Vec<ParfumoNote>) -> bool {
    let key = |name: &str| {
        name.split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .to_lowercase()
    };
    let Some(found) = results
        .into_iter()
        .find(|found| key(&found.name) == key(&note.name))
    else {
        return false;
    };
    let image_url = image_link(&found.image_url);
    if !image_url.is_empty() {
        note.image_url = image_url;
    }
    let parfumo_url = parfumo_link(&found.parfumo_url);
    if !parfumo_url.is_empty() {
        note.parfumo_url = parfumo_url;
    }
    if !found.categories.is_empty() {
        note.categories = clean_categories(&note.name, found.categories);
    }
    true
}

async fn enrich_fragrance_notes(app: tauri::AppHandle, draft: &mut ParfumoFragranceDraft) {
    let mut unresolved = Vec::new();
    let mut categories_incomplete = false;
    // Resolve each unique fragrance-page name through the same search used by
    // the Notes library. Do not substitute a similar but different note.
    for batch in draft.notes.chunks_mut(3) {
        let requests: Vec<_> = batch
            .iter()
            .map(|note| {
                let app = app.clone();
                let query = note.name.clone();
                tauri::async_runtime::spawn(async move { search_parfumo_notes(app, query).await })
            })
            .collect();
        for (note, request) in batch.iter_mut().zip(requests) {
            match request.await {
                Ok(Ok(results)) => {
                    if !apply_note_search_result(note, results) {
                        unresolved.push(note.name.clone());
                    }
                }
                _ => unresolved.push(note.name.clone()),
            }
        }
        if populate_note_categories(&app, batch).await.is_err() {
            categories_incomplete = true;
        }
    }
    if !unresolved.is_empty() {
        draft.warnings.push(format!("The Parfumo note search could not resolve: {}. Their names and available details from the fragrance page will still be saved.", unresolved.join(", ")));
    }
    if categories_incomplete {
        draft.warnings.push("Some note categories could not be read from Parfumo. The other note details will still be saved.".into());
    }
}

fn parse_fragrance(body: &str, source_url: String) -> Result<ParfumoFragranceDraft, String> {
    check_fragrance_response(body)?;
    let html = Html::parse_document(body);
    let mut draft = ParfumoFragranceDraft {
        name: field(&html, "h1 [itemprop='name'], h1 .p_name, h1"),
        brand: field(
            &html,
            "[itemprop='brand'], h1 a[href*='/Brands/'], .p_brand a",
        ),
        image_url: image_link(&field(&html, "meta[property='og:image']")),
        description: field(
            &html,
            "[itemprop='description'], .p_description, meta[name='description']",
        ),
        concentration: field(&html, ".p_concentration"),
        source_url,
        ..Default::default()
    };
    // Prefer structured product data when available; do not copy community scores
    // into the user's personal ratings.
    fn product(value: &serde_json::Value) -> Option<&serde_json::Value> {
        if value.get("@type").is_some_and(|t| {
            t.as_str() == Some("Product")
                || t.as_array()
                    .is_some_and(|a| a.iter().any(|t| t.as_str() == Some("Product")))
        }) {
            return Some(value);
        }
        if let Some(items) = value.as_array() {
            return items.iter().find_map(product);
        }
        value.get("@graph").and_then(product)
    }
    for script in html.select(&Selector::parse("script[type='application/ld+json']").unwrap()) {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&script.inner_html()) {
            if let Some(p) = product(&value) {
                if let Some(name) = p.get("name").and_then(|v| v.as_str()) {
                    draft.name = name.trim().to_owned();
                }
                if let Some(brand) = p.get("brand").and_then(|v| {
                    v.as_str()
                        .or_else(|| v.get("name").and_then(|n| n.as_str()))
                }) {
                    draft.brand = brand.trim().to_owned();
                }
                if let Some(image) = p.get("image").and_then(|v| {
                    v.as_str()
                        .or_else(|| v.get(0).and_then(|v| v.as_str()))
                        .or_else(|| v.get("url").and_then(|v| v.as_str()))
                }) {
                    draft.image_url = image_link(image);
                }
                if let Some(description) = p.get("description").and_then(|v| v.as_str()) {
                    draft.description = description.to_owned();
                }
            }
        }
    }
    let year = field(
        &html,
        "[itemprop='releaseDate'], .p_year, a[href*='/Release_Years/']",
    );
    draft.year = year
        .split(|c: char| !c.is_ascii_digit())
        .filter_map(|s| s.parse::<i32>().ok())
        .find(|year| (1800..=2100).contains(year));
    draft.perfumer = html
        .select(&Selector::parse("a[href*='/Perfumers/']").unwrap())
        .map(text_of)
        .filter(|s| !s.is_empty())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>()
        .join(", ");
    let audience = field(&html, "[itemprop='audience'], .p_gender").to_lowercase();
    draft.audience = match audience.as_str() {
        "women" | "female" => "Women",
        "men" | "male" => "Men",
        "unisex" | "women and men" => "Unisex",
        _ => "",
    }
    .into();
    let notes_selector =
        Selector::parse(".pyramid_block .clickable_note_img, a[href*='/Fragrance_Note/']").unwrap();
    // Find the nearest labelled group containing each note. This handles both
    // nested pyramid rows and rows with a preceding heading.
    for note in html.select(&notes_selector) {
        let Some(imported_note) = fragrance_note(note) else {
            continue;
        };
        let name = &imported_note.name;
        // Parfumo uses t/m/b for top/heart/base on its clickable note spans.
        let mut layer = match note.value().attr("data-nt") {
            Some("t") => Some(0),
            Some("m") => Some(1),
            Some("b") => Some(2),
            _ => None,
        };
        for ancestor in note
            .ancestors()
            .filter_map(scraper::ElementRef::wrap)
            .take(5)
        {
            if layer.is_some() {
                break;
            }
            let heading = ancestor
                .select(
                    &Selector::parse("h2, h3, h4, h5, .pyramid_title, .pyramid_level_label, .pyramid-label, .pyramid-level-label, .pyramid-level, .pyramid_level").unwrap(),
                )
                .next()
                .map(text_of)
                .or_else(|| {
                    ancestor
                        .prev_siblings()
                        .filter_map(scraper::ElementRef::wrap)
                        .next()
                        .map(text_of)
                })
                .unwrap_or_default();
            layer = match heading.trim().to_lowercase().as_str() {
                "top notes" => Some(0),
                "heart notes" | "middle notes" => Some(1),
                "base notes" => Some(2),
                _ => None,
            };
            if layer.is_some() {
                break;
            }
        }
        let target = match layer {
            Some(0) => &mut draft.top_notes,
            Some(1) => &mut draft.heart_notes,
            Some(2) => &mut draft.base_notes,
            _ => continue,
        };
        if !target.iter().any(|n| n.eq_ignore_ascii_case(&name)) {
            target.push(name.clone());
        }
        if let Some(existing) = draft
            .notes
            .iter_mut()
            .find(|note| note.name.eq_ignore_ascii_case(name))
        {
            if existing.image_url.is_empty() {
                existing.image_url = imported_note.image_url;
            }
            if existing.parfumo_url.is_empty() {
                existing.parfumo_url = imported_note.parfumo_url;
            }
        } else {
            draft.notes.push(imported_note);
        }
    }
    if draft.name.is_empty() || draft.brand.is_empty() {
        return Err("Could not read the fragrance name and brand from this Parfumo page. Its layout may have changed; you can still add it manually.".into());
    }
    if draft.year.is_none() {
        draft
            .warnings
            .push("Release year was not available. Enter it before saving.".into());
    }
    if draft.top_notes.is_empty() && draft.heart_notes.is_empty() && draft.base_notes.is_empty() {
        draft
            .warnings
            .push("No labelled note pyramid was found. Add the notes manually.".into());
    }
    if draft.image_url.is_empty() {
        draft.warnings.push("No bottle image was found.".into());
    }
    Ok(draft)
}

#[tauri::command]
pub(crate) async fn search_parfumo_fragrances(
    app: tauri::AppHandle,
    query: String,
) -> Result<Vec<ParfumoFragranceResult>, String> {
    let query = query.trim();
    if query.chars().count() < 2 {
        return Ok(Vec::new());
    }
    if query.chars().count() > 100 {
        return Err("Search term is too long (maximum 100 characters).".into());
    }
    let mut url = Url::parse("https://www.parfumo.com/s_perfumes.php").unwrap();
    url.query_pairs_mut().append_pair("filter", query);
    parse_fragrance_results(&fetch_fragrance_page(app, url.to_string()).await?)
}

#[tauri::command]
pub(crate) async fn import_parfumo_fragrance(
    app: tauri::AppHandle,
    url: String,
) -> Result<ParfumoFragranceDraft, String> {
    let url = fragrance_url(&url)?;
    let body = fetch_fragrance_page(app.clone(), url.clone()).await?;
    let mut draft = parse_fragrance(&body, url)?;
    enrich_fragrance_notes(app, &mut draft).await;
    Ok(draft)
}

#[cfg(test)]
mod fragrance_tests {
    use super::*;

    fn note_database() -> rusqlite::Connection {
        let mut conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "PRAGMA foreign_keys = ON;
             CREATE TABLE fragrances (id INTEGER PRIMARY KEY, top_notes TEXT, heart_notes TEXT, base_notes TEXT);
             CREATE TABLE note_images (name TEXT PRIMARY KEY COLLATE NOCASE, image_data_url TEXT NOT NULL, parfumo_url TEXT NOT NULL DEFAULT '', categories TEXT NOT NULL DEFAULT '[]');",
        ).unwrap();
        crate::ensure_fragrance_notes_schema(&mut conn).unwrap();
        conn
    }

    #[test]
    fn parses_the_supplied_clickable_pyramid_and_saves_all_ten_notes() {
        let body = format!(
            "<h1>Test</h1><span itemprop='brand'>House</span>{}",
            include_str!("../tests/fixtures/parfumo-pyramid.html")
        );
        let mut draft = parse_fragrance(&body, String::new()).unwrap();
        assert_eq!(
            draft.top_notes,
            ["Pineapple", "Granny Smith apple", "Mandarin orange"]
        );
        assert_eq!(draft.heart_notes, ["Vanilla", "Cedarwood", "Oakmoss"]);
        assert_eq!(
            draft.base_notes,
            ["Ambergris", "Dry woods", "Musk", "Caramel"]
        );
        assert_eq!(draft.notes.len(), 10);
        assert_eq!(
            draft.notes[0].image_url,
            "https://media.parfumo.com/notes_png/21/21_c8b2e7a88752251d10a5ada6a0626047683a6409_1200.png"
        );
        assert!(draft.notes.iter().all(|note| note.parfumo_url.is_empty()));
        assert!(draft.notes.iter().all(|note| {
            note.image_url
                .starts_with("https://media.parfumo.com/notes_png/")
                && !note.image_url.contains('?')
        }));
        assert!(
            !draft
                .warnings
                .iter()
                .any(|warning| warning.contains("note pyramid"))
        );
        assert!(apply_note_search_result(
            &mut draft.notes[0],
            parse_notes(
                r#"[{"name":"Pineapple","url":"/Fragrance_Note/Pineapple","categories":["Fruits"]}]"#
            )
        ));

        let mut conn = note_database();
        let transaction = conn.transaction().unwrap();
        transaction
            .execute("INSERT INTO fragrances VALUES (1, '[]', '[]', '[]')", [])
            .unwrap();
        crate::replace_fragrance_notes(
            &transaction,
            1,
            [&draft.top_notes, &draft.heart_notes, &draft.base_notes],
        )
        .unwrap();
        save_fragrance_notes(
            &transaction,
            [&draft.top_notes, &draft.heart_notes, &draft.base_notes],
            &draft.notes,
        )
        .unwrap();
        transaction.commit().unwrap();
        let saved = crate::load_note_images(&conn).unwrap();
        assert_eq!(saved.len(), 10);
        assert!(
            saved
                .iter()
                .all(|note| !note.image_url.is_empty() && note.usage_count == 1)
        );
        let pineapple = saved.iter().find(|note| note.name == "Pineapple").unwrap();
        assert_eq!(pineapple.categories, ["Fruits"]);
        assert_eq!(
            pineapple.parfumo_url,
            "https://www.parfumo.com/Fragrance_Note/Pineapple"
        );
    }

    #[test]
    fn clickable_notes_can_use_the_pyramid_title_and_image_alt_text() {
        let body = r#"<h1>Test</h1><span itemprop="brand">House</span><div class="pyramid_block"><div class="pyramid_title"><img alt="Heart Notes"> Heart Notes</div><div><span class="clickable_note_img" data-n_id="1894"><span><img src="https://media.parfumo.com/notes_png/musk.png" alt="Musk"></span></span></div></div>"#;
        let draft = parse_fragrance(body, String::new()).unwrap();
        assert_eq!(draft.heart_notes, ["Musk"]);
        assert!(draft.top_notes.is_empty());
        assert!(draft.base_notes.is_empty());
        assert_eq!(
            draft.notes[0].image_url,
            "https://media.parfumo.com/notes_png/musk.png"
        );
    }

    #[test]
    fn retains_fragrance_page_note_details_for_search() {
        let body = r#"<h1>Test</h1><span itemprop="brand">House</span>
          <div><h3>Top Notes</h3><a href="/Fragrance_Note/Rose"><img src="/img/placeholder.png" data-src="https://media.parfumo.com/notes_png/rose.png?width=40" alt="Rose"></a></div>
          <div><h3>Heart Notes</h3><div><img src="https://media.parfumo.com/notes_png/musk.png"><a href="/Fragrance_Note/Musk">Musk</a></div><a href="/Fragrance_Note/Rose">Rose</a></div>
          <div><h3>Base Notes</h3><a href="https://evil.example/Fragrance_Note/Bad">Bad</a></div>"#;
        let draft = parse_fragrance(body, String::new()).unwrap();
        assert_eq!(draft.top_notes, ["Rose"]);
        assert_eq!(draft.heart_notes, ["Musk", "Rose"]);
        assert!(draft.base_notes.is_empty());
        assert_eq!(draft.notes.len(), 2);
        assert_eq!(
            draft.notes[0].image_url,
            "https://media.parfumo.com/notes_png/rose.png"
        );
        assert_eq!(
            draft.notes[0].parfumo_url,
            "https://www.parfumo.com/Fragrance_Note/Rose"
        );
        assert_eq!(
            draft.notes[1].image_url,
            "https://media.parfumo.com/notes_png/musk.png"
        );
    }

    #[test]
    fn resolves_the_exact_note_and_keeps_the_fragrance_page_name() {
        let mut note = ParfumoNote {
            name: "White musk".into(),
            image_url: String::new(),
            parfumo_url: "/Fragrance_Note/Original".into(),
            categories: Vec::new(),
        };
        let results = parse_notes(
            r#"[{"name":"White musk rose","image":"/img/wrong.png"},{"name":" WHITE   MUSK ","image":"https://media.parfumo.com/notes_png/musk.png?width=40","url":"/Fragrance_Note/White_Musk","categories":["Animalic Notes","ScentMix"]}]"#,
        );
        assert!(apply_note_search_result(&mut note, results));
        assert_eq!(note.name, "White musk");
        assert_eq!(
            note.image_url,
            "https://media.parfumo.com/notes_png/musk.png"
        );
        assert_eq!(
            note.parfumo_url,
            "https://www.parfumo.com/Fragrance_Note/White_Musk"
        );
        assert_eq!(note.categories, ["Animalic Notes"]);

        // A missing field in the search result must not erase page data.
        assert!(apply_note_search_result(
            &mut note,
            parse_notes(r#"[{"name":"White musk"}]"#)
        ));
        assert_eq!(
            note.image_url,
            "https://media.parfumo.com/notes_png/musk.png"
        );
        assert_eq!(note.categories, ["Animalic Notes"]);
    }

    #[test]
    fn unresolved_search_keeps_page_details_without_using_a_similar_note() {
        let mut note = ParfumoNote {
            name: "Rose".into(),
            image_url: "https://media.parfumo.com/notes_png/rose.png".into(),
            parfumo_url: "https://www.parfumo.com/Fragrance_Note/Rose".into(),
            categories: Vec::new(),
        };
        assert!(!apply_note_search_result(
            &mut note,
            parse_notes(
                r#"[{"name":"Rosewood","image":"/img/rosewood.png","url":"/Fragrance_Note/Rosewood"}]"#
            )
        ));
        assert!(!apply_note_search_result(&mut note, Vec::new()));
        assert_eq!(note.name, "Rose");
        assert_eq!(
            note.image_url,
            "https://media.parfumo.com/notes_png/rose.png"
        );
        assert_eq!(
            note.parfumo_url,
            "https://www.parfumo.com/Fragrance_Note/Rose"
        );
    }

    #[test]
    fn fragrance_page_notes_populate_the_library_from_note_search_results() {
        let mut draft = parse_fragrance(r#"<h1>Test</h1><span itemprop="brand">House</span><div class="pyramid_block"><span class="clickable_note_img" data-nt="t" data-n_id="1">Lavender</span></div><div class="pyramid_block"><span class="clickable_note_img" data-nt="m" data-n_id="1">Lavender</span></div>"#, String::new()).unwrap();
        assert_eq!(draft.notes.len(), 1);
        assert!(draft.notes[0].image_url.is_empty());
        assert!(draft.notes[0].parfumo_url.is_empty());
        let results = parse_notes(
            r#"<table><tr><td><div data-name_url="Lavender"><img src="https://media.parfumo.com/notes_png/lavender.png?width=40">Lavender</div></td></tr></table>"#,
        );
        assert!(apply_note_search_result(&mut draft.notes[0], results));
        draft.notes[0].categories = categories_from_html(
            r#"<a href="/Fragrance_Note/Lavender">Lavender</a><a href="/Fragrance_Notes/Herbaceous_Notes">Herbaceous Notes</a>"#,
            "Lavender",
        );

        let mut conn = note_database();
        let transaction = conn.transaction().unwrap();
        transaction
            .execute("INSERT INTO fragrances VALUES (1, '[]', '[]', '[]')", [])
            .unwrap();
        crate::replace_fragrance_notes(
            &transaction,
            1,
            [&draft.top_notes, &draft.heart_notes, &draft.base_notes],
        )
        .unwrap();
        save_fragrance_notes(
            &transaction,
            [&draft.top_notes, &draft.heart_notes, &draft.base_notes],
            &draft.notes,
        )
        .unwrap();
        transaction.commit().unwrap();
        let notes = crate::load_note_images(&conn).unwrap();
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].name, "Lavender");
        assert_eq!(
            notes[0].image_url,
            "https://media.parfumo.com/notes_png/lavender.png"
        );
        assert_eq!(
            notes[0].parfumo_url,
            "https://www.parfumo.com/Fragrance_Note/Lavender"
        );
        assert_eq!(notes[0].categories, ["Herbaceous Notes"]);
        assert_eq!(notes[0].usage_count, 1);
    }

    #[test]
    fn saves_imported_notes_once_and_preserves_existing_details() {
        let mut conn = note_database();
        conn.execute("INSERT INTO note_images (name, image_data_url, categories) VALUES ('Rose', 'https://example.com/my-rose.png', '[\"Flowers\"]')", []).unwrap();
        let mut notes = parse_notes(
            r#"[{"name":"rose","image":"https://media.parfumo.com/notes_png/rose.png","url":"/Fragrance_Note/Rose","categories":["Floral Notes"]},{"name":"Musk","image":"https://media.parfumo.com/notes_png/musk.png?width=40","url":"/Fragrance_Note/Musk","categories":["Animalic Notes"]},{"name":"Removed","url":"/Fragrance_Note/Removed"}]"#,
        );
        notes.push(notes[1].clone());
        let top = vec!["rose".into(), "Pepper".into()];
        let heart = vec!["Rose".into()];
        let base = vec!["Musk".into()];
        let transaction = conn.transaction().unwrap();
        transaction
            .execute("INSERT INTO fragrances VALUES (1, '[]', '[]', '[]')", [])
            .unwrap();
        crate::replace_fragrance_notes(&transaction, 1, [&top, &heart, &base]).unwrap();
        save_fragrance_notes(&transaction, [&top, &heart, &base], &notes).unwrap();
        transaction.commit().unwrap();

        let saved = crate::load_note_images(&conn).unwrap();
        assert_eq!(saved.len(), 3);
        let rose = saved.iter().find(|note| note.name == "Rose").unwrap();
        assert_eq!(rose.image_url, "https://example.com/my-rose.png");
        assert_eq!(rose.categories, ["Flowers"]);
        assert_eq!(
            rose.parfumo_url,
            "https://www.parfumo.com/Fragrance_Note/Rose"
        );
        assert_eq!(rose.usage_count, 1);
        let musk = saved.iter().find(|note| note.name == "Musk").unwrap();
        assert_eq!(
            musk.image_url,
            "https://media.parfumo.com/notes_png/musk.png"
        );
        assert_eq!(musk.categories, ["Animalic Notes"]);
        assert_eq!(
            musk.parfumo_url,
            "https://www.parfumo.com/Fragrance_Note/Musk"
        );
        assert_eq!(musk.usage_count, 1);

        // Importing into an existing fragrance also fills missing metadata.
        let transaction = conn.transaction().unwrap();
        notes.push(ParfumoNote {
            name: "Pepper".into(),
            image_url: "https://media.parfumo.com/notes_png/pepper.png".into(),
            parfumo_url: "/Fragrance_Note/Pepper".into(),
            categories: vec!["Spices".into()],
        });
        crate::replace_fragrance_notes(&transaction, 1, [&top, &heart, &base]).unwrap();
        save_fragrance_notes(&transaction, [&top, &heart, &base], &notes).unwrap();
        transaction.commit().unwrap();
        let pepper = crate::load_note_image(&conn, "Pepper").unwrap();
        assert_eq!(pepper.categories, ["Spices"]);
        assert_eq!(pepper.usage_count, 1);
    }

    #[test]
    fn failed_note_save_rolls_back_the_fragrance_and_note_records() {
        let mut conn = note_database();
        let top = vec!["Rose".into(), "Invalid".into()];
        let empty = vec![];
        let notes = parse_notes(
            r#"[{"name":"Rose","image":"https://media.parfumo.com/notes_png/rose.png"},{"name":"Invalid"}]"#,
        );
        let mut notes = notes;
        notes[1].image_url = "javascript:alert(1)".into();
        {
            let transaction = conn.transaction().unwrap();
            transaction
                .execute("INSERT INTO fragrances VALUES (1, '[]', '[]', '[]')", [])
                .unwrap();
            crate::replace_fragrance_notes(&transaction, 1, [&top, &empty, &empty]).unwrap();
            assert!(save_fragrance_notes(&transaction, [&top, &empty, &empty], &notes).is_err());
        }
        assert!(crate::load_note_images(&conn).unwrap().is_empty());
        for table in ["fragrances", "fragrance_notes"] {
            let count: i64 = conn
                .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                    row.get(0)
                })
                .unwrap();
            assert_eq!(count, 0);
        }
    }

    #[test]
    fn rejects_untrusted_and_non_fragrance_urls() {
        for url in [
            "https://evil.com/Perfumes/A/B",
            "https://parfumo.com.evil.com/Perfumes/A/B",
            "/Brands/Dior",
            "http://parfumo.com/Perfumes/A/B",
        ] {
            assert!(fragrance_url(url).is_err());
        }
        assert_eq!(
            fragrance_url("/Perfumes/Dior/Sauvage").unwrap(),
            "https://www.parfumo.com/Perfumes/Dior/Sauvage"
        );
    }
    #[test]
    fn search_deduplicates_and_ignores_navigation() {
        let results = parse_fragrance_results(r#"<a href="/Brands/Dior">Dior</a><a href="/Perfumes/Dior/Sauvage"><img alt="Sauvage" data-src="https://media.parfumo.com/a.jpg"></a><a href="/Perfumes/Dior/Sauvage">Sauvage</a>"#).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "Sauvage");
    }
    #[test]
    fn imports_product_and_labelled_pyramid_without_inventing_missing_fields() {
        let body = r#"<script type="application/ld+json">{"@graph":[{"@type":"Product","name":"Test","brand":{"name":"House"},"image":["https://media.parfumo.com/bottle.jpg"]}]}</script><a href="/Release_Years/2020">2020</a><div><h3>Top Notes</h3><a href="/Fragrance_Note/Bergamot">Bergamot</a></div><div><h3>Heart Notes</h3><a href="/Fragrance_Note/Rose">Rose</a></div><div><h3>Base Notes</h3><a href="/Fragrance_Note/Musk">Musk</a></div>"#;
        let draft =
            parse_fragrance(body, "https://www.parfumo.com/Perfumes/House/Test".into()).unwrap();
        assert_eq!(draft.name, "Test");
        assert_eq!(draft.brand, "House");
        assert_eq!(draft.year, Some(2020));
        assert_eq!(draft.top_notes, ["Bergamot"]);
        assert_eq!(draft.heart_notes, ["Rose"]);
        assert_eq!(draft.base_notes, ["Musk"]);
        assert!(draft.audience.is_empty());
        assert!(draft.warnings.is_empty());
        assert!(parse_fragrance("<html>Unknown page</html>", String::new()).is_err());
        assert!(parse_fragrance_results("Just a moment...").is_err());
    }
}
