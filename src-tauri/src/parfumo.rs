use serde::{Deserialize, Serialize};
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};
use url::Url;

const LOGIN_WINDOW: &str = "parfumo-login";
const KEYRING_SERVICE: &str = "com.scentkeep.app.parfumo";
const KEYRING_USER: &str = "session";
const PARFUMO_URL: &str = "https://www.parfumo.com/";

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
