use std::path::{Path, PathBuf};
use std::time::Duration;

use tauri::{AppHandle, Manager, State, WebviewWindow};
use tauri_plugin_dialog::DialogExt;

use crate::clipboard::read_doc;
use crate::doc_windows::{DocWindows, join_result, open_doc, open_paths};
use crate::error::{AppError, report};
use crate::export;
use crate::file_id::DocKey;
use crate::i18n;
use crate::import::file_name;
use crate::model::Doc;
use crate::positions::{PositionsState, now_ms};
use crate::prefs::{self, Prefs, PrefsState};

#[tauri::command]
pub async fn take_pending_document(app: AppHandle, window: WebviewWindow) -> Option<Doc> {
    app.state::<DocWindows>().slots.lock().unwrap().take(window.label())
}

#[tauri::command]
pub async fn set_position(app: AppHandle, window: WebviewWindow, char_index: u32) {
    let key = app.state::<DocWindows>().slots.lock().unwrap().key(window.label());
    let Some(key) = key else { return };
    if app.state::<PositionsState>().record(&key, char_index, now_ms()) {
        let app = app.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_secs(5));
            app.state::<PositionsState>().scheduled_flush();
        });
    }
}

#[tauri::command]
pub async fn open_from_clipboard(app: AppHandle, window: WebviewWindow) {
    let doc = match tauri::async_runtime::spawn_blocking(read_doc).await {
        Ok(doc) => doc,
        Err(err) => {
            log::error!("lectura del portapapeles: {err}");
            return;
        }
    };
    let Some(doc) = doc else { return };
    if let Err(err) = open_doc(&app, window.label(), doc, None) {
        report(&app, Some(&window), &err, "portapapeles");
    }
}

#[tauri::command]
pub async fn open_dialog(app: AppHandle, window: WebviewWindow) {
    let (dialog_app, dialog_window) = (app.clone(), window.clone());
    let picked = tauri::async_runtime::spawn_blocking(move || -> Result<Option<PathBuf>, AppError> {
        let Some(file) = dialog_app
            .dialog()
            .file()
            .set_parent(&dialog_window)
            .add_filter(i18n::t("filter.documents"), &["txt", "rtf", "pdf", "html", "htm", "xhtml", "docx", "epub"])
            .add_filter(i18n::t("filter.all"), &["*"])
            .blocking_pick_file()
        else {
            return Ok(None);
        };
        file.into_path()
            .map(Some)
            .map_err(|err| AppError::Internal(err.to_string()))
    })
    .await;
    match join_result(picked) {
        Ok(Some(path)) => open_paths(app, window.label().to_owned(), vec![path]).await,
        Ok(None) => {}
        Err(err) => report(&app, Some(&window), &err, "diálogo de apertura"),
    }
}

#[tauri::command]
pub async fn save_copy(app: AppHandle, window: WebviewWindow) {
    let (doc, clave) = {
        let state = app.state::<DocWindows>();
        let slots = state.slots.lock().unwrap();
        (slots.doc(window.label()), slots.key(window.label()))
    };
    let Some(doc) = doc else { return };
    let (dialog_app, dialog_window) = (app.clone(), window.clone());
    let saved = tauri::async_runtime::spawn_blocking(move || -> Result<(), AppError> {
        let nombre = nombre_propuesto(clave.as_ref(), &doc.title);
        let Some(file) = dialog_app
            .dialog()
            .file()
            .set_parent(&dialog_window)
            .add_filter(i18n::t("filter.text"), &["txt"])
            .add_filter("RTF", &["rtf"])
            .set_file_name(nombre)
            .blocking_save_file()
        else {
            return Ok(());
        };
        let path = file
            .into_path()
            .map_err(|err| AppError::Internal(err.to_string()))?;
        let texto = if path.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("rtf")) {
            export::rtf(&doc)
        } else {
            export::plain(&doc)
        };
        std::fs::write(&path, texto).map_err(|source| AppError::Write {
            file: file_name(&path),
            source,
        })
    })
    .await;
    if let Err(err) = join_result(saved) {
        report(&app, Some(&window), &err, "guardar copia");
    }
}

fn nombre_propuesto(clave: Option<&DocKey>, titulo: &str) -> String {
    match clave {
        Some(clave) => Path::new(&clave.path)
            .file_stem()
            .map_or_else(|| titulo.to_owned(), |stem| stem.to_string_lossy().into_owned()),
        None => titulo.to_owned(),
    }
}

#[tauri::command]
pub fn get_prefs(state: State<'_, PrefsState>) -> Prefs {
    state.prefs.lock().unwrap().clone()
}

#[tauri::command]
pub fn set_prefs(app: AppHandle, prefs: Prefs) -> Prefs {
    prefs::apply(&app, prefs)
}

#[tauri::command]
// Restablecer no toca el idioma: está en la sección de texto y colores.
pub fn reset_prefs(app: AppHandle, state: State<'_, PrefsState>) -> Prefs {
    let language = state.prefs.lock().unwrap().language.clone();
    prefs::apply(&app, Prefs { language, ..Prefs::default() })
}

#[tauri::command]
pub async fn list_fonts() -> Vec<String> {
    tauri::async_runtime::spawn_blocking(crate::fonts::system_families)
        .await
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clave(path: &str) -> DocKey {
        DocKey {
            file_id: None,
            content_hash: 0,
            path: path.to_owned(),
        }
    }

    #[test]
    fn con_clave_propone_el_nombre_del_archivo() {
        let clave = clave("/libros/dr-no.epub");
        assert_eq!(nombre_propuesto(Some(&clave), "Dr. No"), "dr-no");
    }

    #[test]
    fn sin_clave_propone_el_titulo_entero() {
        assert_eq!(nombre_propuesto(None, "Dr. No"), "Dr. No");
    }

    #[test]
    fn con_clave_con_punto_corta_solo_la_extension() {
        let clave = clave("/libros/tomo.v2.epub");
        assert_eq!(nombre_propuesto(Some(&clave), "Tomo 2"), "tomo.v2");
    }
}
