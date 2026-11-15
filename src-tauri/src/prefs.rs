use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use serde::{Deserialize, Serialize};
use tauri::window::Color;
use tauri::{AppHandle, Emitter, Manager};
use ts_rs::TS;

use crate::error::{AppError, report};
use crate::i18n;
use crate::store;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, TS)]
#[ts(export)]
#[serde(default)]
pub struct Prefs {
    pub font_family: String,
    pub font_size: f64,
    pub column_width: f64,
    pub relative_inset: f64,
    pub vertical_inset: f64,
    pub line_height: f64,
    pub background: String,
    pub foreground: String,
    pub language: String,
}

impl Default for Prefs {
    fn default() -> Self {
        Self {
            font_family: "Literata".into(),
            font_size: 14.0,
            column_width: 360.0,
            relative_inset: 3.0,
            vertical_inset: 20.0,
            line_height: 1.1,
            background: "#ffffff".into(),
            foreground: "#000000".into(),
            language: "auto".into(),
        }
    }
}

impl Prefs {
    pub fn validated(self) -> Self {
        let d = Self::default();
        Self {
            font_family: familia_valida(&self.font_family).unwrap_or(d.font_family),
            font_size: recorte(self.font_size, 8.0, 48.0, d.font_size),
            column_width: recorte(self.column_width, 200.0, 800.0, d.column_width),
            relative_inset: recorte(self.relative_inset, 0.0, 20.0, d.relative_inset),
            vertical_inset: recorte(self.vertical_inset, 0.0, 80.0, d.vertical_inset),
            line_height: recorte(self.line_height, 1.0, 2.0, d.line_height),
            background: color_hex(&self.background).unwrap_or(d.background),
            foreground: color_hex(&self.foreground).unwrap_or(d.foreground),
            language: if i18n::is_valid(&self.language) { self.language } else { d.language },
        }
    }
}

fn recorte(valor: f64, min: f64, max: f64, defecto: f64) -> f64 {
    if valor.is_finite() {
        valor.clamp(min, max)
    } else {
        defecto
    }
}

pub fn familia_valida(raw: &str) -> Option<String> {
    let nombre = raw.trim();
    let longitud = nombre.chars().count();
    let valida = (1..=100).contains(&longitud)
        && !nombre.contains(['"', ';', '{', '}', '<', '>', '\\']);
    valida.then(|| nombre.to_owned())
}

fn color_hex(raw: &str) -> Option<String> {
    let hex = raw.strip_prefix('#')?;
    let valido = hex.len() == 6 && hex.bytes().all(|b| b.is_ascii_hexdigit());
    valido.then(|| format!("#{}", hex.to_ascii_lowercase()))
}

pub fn color(hex: &str) -> Color {
    let canal = |i: usize| {
        hex.get(i..i + 2)
            .and_then(|s| u8::from_str_radix(s, 16).ok())
            .unwrap_or(0)
    };
    Color(canal(1), canal(3), canal(5), 255)
}

pub struct PrefsState {
    pub prefs: Mutex<Prefs>,
    path: PathBuf,
    warned: AtomicBool,
}

impl PrefsState {
    pub fn load(path: PathBuf) -> Self {
        let prefs = store::load(&path, |_: &Prefs| true).validated();
        Self {
            prefs: Mutex::new(prefs),
            path,
            warned: AtomicBool::new(false),
        }
    }
}

pub fn apply(app: &AppHandle, new: Prefs) -> Prefs {
    let state = app.state::<PrefsState>();
    let prefs = new.validated();
    *state.prefs.lock().unwrap() = prefs.clone();
    i18n::set(i18n::effective(&prefs.language));
    match store::save(&state.path, &prefs) {
        Ok(()) => {}
        Err(source) if !state.warned.swap(true, Ordering::Relaxed) => {
            report(app, None, &AppError::Persist { source }, "prefs.json");
        }
        Err(source) => log::error!("prefs.json: {source}"),
    }
    let fondo = color(&prefs.background);
    for (label, window) in app.webview_windows() {
        if label.starts_with("reader-")
            && let Err(err) = window.set_background_color(Some(fondo))
        {
            log::warn!("color de fondo de {label}: {err}");
        }
    }
    if let Err(err) = app.emit("prefs-changed", &prefs) {
        log::error!("prefs-changed: {err}");
    }
    prefs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_son_los_de_la_especificacion() {
        let d = Prefs::default();
        assert_eq!(d.font_family, "Literata");
        assert_eq!(d.font_size, 14.0);
        assert_eq!(d.column_width, 360.0);
        assert_eq!(d.relative_inset, 3.0);
        assert_eq!(d.vertical_inset, 20.0);
        assert_eq!(d.line_height, 1.1);
        assert_eq!(d.background, "#ffffff");
        assert_eq!(d.foreground, "#000000");
        assert_eq!(d.language, "auto");
    }

    #[test]
    fn validated_recorta_en_ambos_extremos() {
        let bajo = Prefs {
            font_size: 1.0,
            column_width: 1.0,
            relative_inset: -5.0,
            vertical_inset: -1.0,
            line_height: 0.1,
            ..Prefs::default()
        }
        .validated();
        assert_eq!(bajo.font_size, 8.0);
        assert_eq!(bajo.column_width, 200.0);
        assert_eq!(bajo.relative_inset, 0.0);
        assert_eq!(bajo.vertical_inset, 0.0);
        assert_eq!(bajo.line_height, 1.0);

        let alto = Prefs {
            font_size: 999.0,
            column_width: 9999.0,
            relative_inset: 50.0,
            vertical_inset: 200.0,
            line_height: 5.0,
            ..Prefs::default()
        }
        .validated();
        assert_eq!(alto.font_size, 48.0);
        assert_eq!(alto.column_width, 800.0);
        assert_eq!(alto.relative_inset, 20.0);
        assert_eq!(alto.vertical_inset, 80.0);
        assert_eq!(alto.line_height, 2.0);
    }

    #[test]
    fn validated_no_finito_vuelve_al_defecto_del_campo() {
        let p = Prefs {
            font_size: f64::NAN,
            line_height: f64::INFINITY,
            ..Prefs::default()
        }
        .validated();
        assert_eq!(p.font_size, 14.0);
        assert_eq!(p.line_height, 1.1);
    }

    #[test]
    fn validated_repara_colores() {
        let d = Prefs::default();
        let malos = |c: &str| {
            Prefs {
                background: c.into(),
                foreground: c.into(),
                ..Prefs::default()
            }
            .validated()
        };
        for c in ["red", "#12345", "#GGGGGG"] {
            let p = malos(c);
            assert_eq!(p.background, d.background, "{c}");
            assert_eq!(p.foreground, d.foreground, "{c}");
        }
        let p = malos("#ABCDEF");
        assert_eq!(p.background, "#abcdef");
        assert_eq!(p.foreground, "#abcdef");
    }

    #[test]
    fn validated_repara_familias() {
        let con = |f: &str| {
            Prefs {
                font_family: f.into(),
                ..Prefs::default()
            }
            .validated()
            .font_family
        };
        let d = Prefs::default().font_family;
        assert_eq!(con(""), d);
        assert_eq!(con(&"a".repeat(101)), d);
        assert_eq!(con("Georgia;"), d);
        assert_eq!(con("\"Georgia"), d);
        assert_eq!(con("Geo\\rgia"), d);
        assert_eq!(con(&"a".repeat(100)), "a".repeat(100));
        assert_eq!(con("  Georgia "), "Georgia");
    }

    #[test]
    fn json_parcial_usa_defecto_en_lo_que_falta() {
        let p: Prefs = serde_json::from_str(r#"{"font_size": 18}"#).unwrap();
        assert_eq!(p.font_size, 18.0);
        assert_eq!(p.column_width, Prefs::default().column_width);
        assert_eq!(p.language, "auto");
    }

    #[test]
    fn validated_repara_idioma() {
        let con = |l: &str| Prefs { language: l.into(), ..Prefs::default() }.validated().language;
        assert_eq!(con("ru"), "ru");
        assert_eq!(con("auto"), "auto");
        assert_eq!(con("pt-BR"), "auto");
        assert_eq!(con(""), "auto");
    }
}
