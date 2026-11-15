use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use tauri::http::header::{CONTENT_TYPE, HeaderValue};
use tauri::http::{Response, StatusCode};
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

use crate::error::{AppError, report_batch};
use crate::file_id::DocKey;
use crate::import::{file_name, from_bytes, read};
use crate::model::Doc;
use crate::positions::PositionsState;
use crate::prefs::{self, PrefsState};

#[derive(Debug, PartialEq)]
pub enum Route {
    Focus(String),
    Deliver(String),
    NewWindow(String),
}

enum Slot {
    Pending(Doc, Option<DocKey>),
    Open(Doc, Option<DocKey>),
}

impl Slot {
    fn key(&self) -> Option<&DocKey> {
        match self {
            Self::Pending(_, key) | Self::Open(_, key) => key.as_ref(),
        }
    }
}

#[derive(Default)]
pub struct Slots(HashMap<String, Slot>);

impl Slots {
    pub fn route(
        &mut self,
        origin: Option<&str>,
        doc: Doc,
        key: Option<DocKey>,
        new_label: &str,
    ) -> Route {
        if let Some(key) = &key {
            let same = self
                .0
                .iter()
                .find(|(_, slot)| slot.key().is_some_and(|k| k.same_file(key)))
                .map(|(label, _)| label.clone());
            if let Some(label) = same {
                return Route::Focus(label);
            }
        }
        match origin {
            Some(origin) if !self.0.contains_key(origin) => {
                self.0.insert(origin.to_owned(), Slot::Pending(doc, key));
                Route::Deliver(origin.to_owned())
            }
            _ => {
                self.0.insert(new_label.to_owned(), Slot::Pending(doc, key));
                Route::NewWindow(new_label.to_owned())
            }
        }
    }

    pub fn take(&mut self, label: &str) -> Option<Doc> {
        match self.0.remove(label)? {
            Slot::Pending(doc, key) => {
                let entregado = doc.clone();
                self.0.insert(label.to_owned(), Slot::Open(doc, key));
                Some(entregado)
            }
            open @ Slot::Open(..) => {
                self.0.insert(label.to_owned(), open);
                None
            }
        }
    }

    pub fn doc(&self, label: &str) -> Option<Doc> {
        match self.0.get(label)? {
            Slot::Pending(doc, _) | Slot::Open(doc, _) => Some(doc.clone()),
        }
    }

    pub fn key(&self, label: &str) -> Option<DocKey> {
        self.0.get(label)?.key().cloned()
    }

    pub fn image(&self, label: &str, id: u32) -> Option<(&'static str, Arc<[u8]>)> {
        let doc = match self.0.get(label)? {
            Slot::Pending(doc, _) | Slot::Open(doc, _) => doc,
        };
        doc.images
            .iter()
            .find(|image| image.id == id)
            .map(|image| (image.mime, Arc::clone(&image.bytes)))
    }

    pub fn close(&mut self, label: &str) {
        self.0.remove(label);
    }
}

pub struct DocWindows {
    pub slots: Mutex<Slots>,
    pub focused: Mutex<String>,
    next: AtomicU32,
}

impl DocWindows {
    pub fn new() -> Self {
        Self {
            slots: Mutex::default(),
            focused: Mutex::default(),
            next: AtomicU32::new(1),
        }
    }

    pub fn new_label(&self) -> String {
        format!("reader-{}", self.next.fetch_add(1, Ordering::Relaxed))
    }
}

pub fn image_response(slots: &Slots, label: &str, path: &str) -> Response<Vec<u8>> {
    let id = path.strip_prefix('/').unwrap_or(path).parse().ok();
    let mut response = Response::new(Vec::new());
    match id.and_then(|id| slots.image(label, id)) {
        Some((mime, bytes)) => {
            response
                .headers_mut()
                .insert(CONTENT_TYPE, HeaderValue::from_static(mime));
            *response.body_mut() = bytes.to_vec();
        }
        None => *response.status_mut() = StatusCode::NOT_FOUND,
    }
    response
}

pub fn pick_reader<'a>(focused: &str, alive: impl IntoIterator<Item = &'a str>) -> Option<String> {
    let alive: Vec<&str> = alive.into_iter().collect();
    if alive.contains(&focused) {
        return Some(focused.to_owned());
    }
    alive
        .into_iter()
        .find(|label| label.starts_with("reader-"))
        .map(str::to_owned)
}

pub fn last_reader(app: &AppHandle) -> Option<String> {
    let state = app.try_state::<DocWindows>()?;
    let focused = state.focused.lock().unwrap_or_else(PoisonError::into_inner).clone();
    let windows = app.webview_windows();
    pick_reader(&focused, windows.keys().map(String::as_str))
}

pub fn focus(app: &AppHandle, label: &str) -> Result<(), AppError> {
    let window = window_of(app, label)?;
    window.unminimize().and_then(|()| window.set_focus()).map_err(internal)
}

pub fn create_reader(app: &AppHandle, label: &str, title: &str) -> Result<WebviewWindow, AppError> {
    let prefs = app.state::<PrefsState>().prefs.lock().unwrap().clone();
    // Prefs inyectadas antes del bundle: el primer fotograma ya sale con ellas, sin IPC.
    let script = serde_json::to_string(&prefs)
        .map(|json| format!("window.__PREFS__ = {json}; window.__SYSTEM_LANG__ = \"{}\";", crate::i18n::system()))
        .map_err(|err| AppError::Internal(err.to_string()))?;
    WebviewWindowBuilder::new(app, label, WebviewUrl::default())
        .title(title)
        .inner_size(1150.0, 640.0)
        .min_inner_size(480.0, 360.0)
        .background_color(prefs::color(&prefs.background))
        .initialization_script(&script)
        .build()
        .map_err(|err| AppError::Internal(err.to_string()))
}

pub fn join_result<T>(joined: tauri::Result<Result<T, AppError>>) -> Result<T, AppError> {
    joined.unwrap_or_else(|err| Err(AppError::Internal(err.to_string())))
}

fn internal(err: tauri::Error) -> AppError {
    AppError::Internal(err.to_string())
}

fn window_of(app: &AppHandle, label: &str) -> Result<WebviewWindow, AppError> {
    app.get_webview_window(label)
        .ok_or_else(|| AppError::Internal(format!("no existe la ventana {label}")))
}

pub fn open_doc(
    app: &AppHandle,
    origin: &str,
    mut doc: Doc,
    key: Option<DocKey>,
) -> Result<(), AppError> {
    if let Some(k) = &key {
        doc.initial_char_index = app.state::<PositionsState>().get(k, doc.flat_len());
    }
    let state = app.state::<DocWindows>();
    let origin_alive = app.get_webview_window(origin).is_some();
    let label = state.new_label();
    let title = doc.title.clone();
    let index = doc.initial_char_index;
    let route = state
        .slots
        .lock()
        .unwrap()
        .route(origin_alive.then_some(origin), doc, key, &label);
    let target = match &route {
        Route::Focus(t) | Route::Deliver(t) | Route::NewWindow(t) => t,
    };
    log::info!("{target}: «{title}» en {index}");
    match route {
        Route::Focus(target) => focus(app, &target),
        Route::Deliver(target) => {
            app.emit_to(target.as_str(), "document-ready", ()).map_err(internal)?;
            window_of(app, &target)?.set_title(&title).map_err(internal)
        }
        Route::NewWindow(target) => create_reader(app, &target, &title)
            .map(|_| ())
            .inspect_err(|_| state.slots.lock().unwrap().close(&target)),
    }
}

pub async fn open_paths(app: AppHandle, origin: String, paths: Vec<PathBuf>) {
    let mut errs = Vec::new();
    for path in paths {
        if path.is_dir() {
            log::info!("se ignora el directorio {}", path.display());
            continue;
        }
        let detail = path.display().to_string();
        let loaded = tauri::async_runtime::spawn_blocking(move || {
            let bytes = read(&path)?;
            let key = DocKey::new(&path, &bytes);
            let doc = from_bytes(&file_name(&path), &bytes)?;
            Ok::<_, AppError>((doc, key))
        })
        .await;
        let opened = join_result(loaded)
            .and_then(|(doc, key)| open_doc(&app, &origin, doc, Some(key)));
        if let Err(err) = opened {
            errs.push((detail, err));
        }
    }
    if !errs.is_empty() {
        report_batch(&app, app.get_webview_window(&origin).as_ref(), &errs);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Composition, Image};

    fn doc(title: &str) -> Doc {
        Doc {
            title: title.into(),
            initial_char_index: 0,
            composition: Composition::Text,
            chapters: Vec::new(),
            blocks: Vec::new(),
            images: Vec::new(),
        }
    }

    fn key(file_id: Option<&str>, path: &str) -> DocKey {
        DocKey {
            file_id: file_id.map(str::to_owned),
            content_hash: 0,
            path: path.to_owned(),
        }
    }

    #[test]
    fn ventana_vacia_recibe_el_documento_una_vez() {
        let mut slots = Slots::default();
        let route = slots.route(Some("reader-0"), doc("a.txt"), None, "reader-1");
        assert_eq!(route, Route::Deliver("reader-0".into()));
        assert_eq!(slots.take("reader-0").map(|d| d.title), Some("a.txt".into()));
        assert!(slots.take("reader-0").is_none());
    }

    #[test]
    fn ventana_con_documento_abre_otra_y_el_pendiente_se_recoge_una_vez() {
        let mut slots = Slots::default();
        slots.route(Some("reader-0"), doc("a.txt"), None, "reader-1");
        let route = slots.route(Some("reader-0"), doc("b.txt"), None, "reader-2");
        assert_eq!(route, Route::NewWindow("reader-2".into()));
        assert_eq!(slots.take("reader-2").map(|d| d.title), Some("b.txt".into()));
        assert!(slots.take("reader-2").is_none());
    }

    #[test]
    fn sin_origen_abre_ventana_nueva() {
        let mut slots = Slots::default();
        let route = slots.route(None, doc("a.txt"), None, "reader-1");
        assert_eq!(route, Route::NewWindow("reader-1".into()));
        assert_eq!(slots.take("reader-1").map(|d| d.title), Some("a.txt".into()));
    }

    #[test]
    fn archivo_abierto_enfoca_su_ventana() {
        let mut slots = Slots::default();
        slots.route(Some("reader-0"), doc("a.txt"), Some(key(Some("1"), "/a")), "reader-1");
        slots.take("reader-0");
        let route = slots.route(None, doc("a.txt"), Some(key(Some("1"), "/a")), "reader-2");
        assert_eq!(route, Route::Focus("reader-0".into()));
        assert!(slots.take("reader-2").is_none());
    }

    #[test]
    fn archivo_pendiente_enfoca_su_ventana() {
        let mut slots = Slots::default();
        slots.route(Some("reader-0"), doc("a.txt"), Some(key(Some("1"), "/a")), "reader-1");
        let route = slots.route(None, doc("a.txt"), Some(key(Some("1"), "/a")), "reader-2");
        assert_eq!(route, Route::Focus("reader-0".into()));
    }

    #[test]
    fn misma_ruta_con_otro_file_id_enfoca() {
        let mut slots = Slots::default();
        slots.route(Some("reader-0"), doc("a.txt"), Some(key(Some("1"), "/a")), "reader-1");
        slots.take("reader-0");
        let route = slots.route(None, doc("a.txt"), Some(key(Some("2"), "/a")), "reader-2");
        assert_eq!(route, Route::Focus("reader-0".into()));
    }

    #[test]
    fn mismo_file_id_en_otra_ruta_enfoca() {
        let mut slots = Slots::default();
        slots.route(Some("reader-0"), doc("a.txt"), Some(key(Some("1"), "/a")), "reader-1");
        slots.take("reader-0");
        let route = slots.route(None, doc("a.txt"), Some(key(Some("1"), "/b")), "reader-2");
        assert_eq!(route, Route::Focus("reader-0".into()));
    }

    #[test]
    fn sin_clave_nunca_enfoca() {
        let mut slots = Slots::default();
        let primero = slots.route(Some("reader-0"), doc("a.txt"), None, "reader-1");
        let segundo = slots.route(Some("reader-0"), doc("a.txt"), None, "reader-2");
        assert_eq!(primero, Route::Deliver("reader-0".into()));
        assert_eq!(segundo, Route::NewWindow("reader-2".into()));
    }

    #[test]
    fn take_conserva_la_clave() {
        let mut slots = Slots::default();
        let clave = key(Some("1"), "/a");
        slots.route(Some("reader-0"), doc("a.txt"), Some(clave.clone()), "reader-1");
        assert!(slots.take("reader-0").is_some());
        assert_eq!(slots.key("reader-0"), Some(clave));
    }

    #[test]
    fn doc_sobrevive_a_take_y_desaparece_con_close() {
        let mut slots = Slots::default();
        slots.route(Some("reader-0"), doc("a.txt"), None, "reader-1");
        assert_eq!(slots.doc("reader-0").map(|d| d.title), Some("a.txt".into()));
        assert!(slots.take("reader-0").is_some());
        assert_eq!(slots.doc("reader-0").map(|d| d.title), Some("a.txt".into()));
        slots.close("reader-0");
        assert!(slots.doc("reader-0").is_none());
    }

    #[test]
    fn cerrar_libera_la_etiqueta() {
        let mut slots = Slots::default();
        let route = slots.route(None, doc("a.txt"), None, "reader-1");
        assert_eq!(route, Route::NewWindow("reader-1".into()));
        slots.close("reader-1");
        let route = slots.route(Some("reader-1"), doc("b.txt"), None, "reader-2");
        assert_eq!(route, Route::Deliver("reader-1".into()));
    }

    #[test]
    fn enfocada_viva_se_elige() {
        let vivas = ["reader-0", "reader-1"];
        assert_eq!(pick_reader("reader-1", vivas), Some("reader-1".into()));
    }

    #[test]
    fn enfocada_muerta_cae_en_otra_lectora() {
        let vivas = ["reader-0", "reader-2"];
        assert_eq!(pick_reader("reader-1", vivas), Some("reader-0".into()));
    }

    #[test]
    fn sin_lectoras_vivas_es_none() {
        assert_eq!(pick_reader("reader-1", ["settings"]), None);
        assert_eq!(pick_reader("", std::iter::empty::<&str>()), None);
    }

    #[test]
    fn ignora_labels_que_no_son_lectoras() {
        let vivas = ["settings", "reader-3"];
        assert_eq!(pick_reader("", vivas), Some("reader-3".into()));
    }

    #[test]
    fn pánico_en_la_importación_es_internal() {
        let joined = tauri::async_runtime::block_on(tauri::async_runtime::spawn_blocking(
            || -> Result<Doc, AppError> { panic!("fallo simulado") },
        ));
        assert!(matches!(join_result(joined), Err(AppError::Internal(_))));
    }

    fn con_imagen(title: &str, id: u32) -> Doc {
        let mut doc = doc(title);
        doc.images.push(Image {
            id,
            mime: "image/png",
            bytes: Arc::from(&[1u8, 2, 3][..]),
        });
        doc
    }

    #[test]
    fn imagen_de_la_ventana_se_sirve_con_su_tipo_y_bytes() {
        let mut slots = Slots::default();
        slots.route(Some("reader-0"), con_imagen("a.epub", 42), None, "reader-1");
        let response = image_response(&slots, "reader-0", "/42");
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()[CONTENT_TYPE], "image/png");
        assert_eq!(response.body(), &vec![1, 2, 3]);
    }

    #[test]
    fn imagen_de_documento_abierto_tambien_se_sirve() {
        let mut slots = Slots::default();
        slots.route(Some("reader-0"), con_imagen("a.epub", 42), None, "reader-1");
        slots.take("reader-0");
        let response = image_response(&slots, "reader-0", "/42");
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[test]
    fn id_desconocido_es_404() {
        let mut slots = Slots::default();
        slots.route(Some("reader-0"), con_imagen("a.epub", 42), None, "reader-1");
        let response = image_response(&slots, "reader-0", "/7");
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert!(response.body().is_empty());
    }

    #[test]
    fn path_no_numerico_es_404() {
        let mut slots = Slots::default();
        slots.route(Some("reader-0"), con_imagen("a.epub", 42), None, "reader-1");
        assert_eq!(image_response(&slots, "reader-0", "/abc").status(), StatusCode::NOT_FOUND);
        assert_eq!(image_response(&slots, "reader-0", "/").status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn imagen_de_otra_ventana_es_404() {
        let mut slots = Slots::default();
        slots.route(Some("reader-0"), con_imagen("a.epub", 42), None, "reader-1");
        slots.route(Some("reader-1"), doc("b.txt"), None, "reader-2");
        let response = image_response(&slots, "reader-1", "/42");
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn ventana_cerrada_es_404() {
        let mut slots = Slots::default();
        slots.route(Some("reader-0"), con_imagen("a.epub", 42), None, "reader-1");
        slots.close("reader-0");
        let response = image_response(&slots, "reader-0", "/42");
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
