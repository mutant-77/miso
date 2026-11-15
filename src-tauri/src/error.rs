use std::io;

use serde::ser::{Serialize, SerializeStruct, Serializer};
use tauri::{AppHandle, WebviewWindow};
use tauri_plugin_dialog::{DialogExt, MessageDialogKind};
use thiserror::Error;

use crate::i18n::{self, format, t_in};

#[derive(Debug, Error)]
pub enum AppError {
    #[error("error de lectura: {file}")]
    Read { file: String, source: io::Error },
    #[error("formato no soportado: {file}")]
    Unsupported { file: String },
    #[error("archivo dañado: {file}: {detail}")]
    Corrupt { file: String, detail: String },
    #[error("protegido con contraseña: {file}")]
    Protected { file: String },
    #[error("error al guardar: {source}")]
    Persist { source: io::Error },
    #[error("error de escritura: {file}")]
    Write { file: String, source: io::Error },
    #[error("error interno: {0}")]
    Internal(String),
}

impl AppError {
    fn kind(&self) -> &'static str {
        match self {
            Self::Read { .. } => "read",
            Self::Unsupported { .. } => "unsupported",
            Self::Corrupt { .. } => "corrupt",
            Self::Protected { .. } => "protected",
            Self::Persist { .. } => "persist",
            Self::Write { .. } => "write",
            Self::Internal(_) => "internal",
        }
    }

    pub fn user_message(&self) -> String {
        self.message_in(i18n::current())
    }

    pub fn message_in(&self, lang: &str) -> String {
        let t = |key| t_in(lang, key);
        match self {
            Self::Read { file, source } => {
                format(t("error.read"), &[("file", file), ("reason", &os_message(source))])
            }
            Self::Unsupported { file } => format(t("error.unsupported"), &[("file", file)]),
            Self::Corrupt { file, .. } => format(t("error.corrupt"), &[("file", file)]),
            Self::Protected { file } => format(t("error.protected"), &[("file", file)]),
            Self::Persist { source } => {
                format(t("error.persist"), &[("reason", &os_message(source))])
            }
            Self::Write { file, source } => {
                format(t("error.write"), &[("file", file), ("reason", &os_message(source))])
            }
            Self::Internal(_) => t("error.internal").into(),
        }
    }
}

pub fn os_message(err: &io::Error) -> String {
    let text = err.to_string();
    let corte = text.rfind(" (os error ").filter(|&i| {
        let codigo = &text[i + " (os error ".len()..];
        codigo.strip_suffix(')').is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
    });
    match corte {
        Some(i) => text[..i].to_owned(),
        None => text,
    }
}

impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut state = serializer.serialize_struct("AppError", 2)?;
        state.serialize_field("kind", self.kind())?;
        state.serialize_field("message", &self.user_message())?;
        state.end()
    }
}

pub fn batch_message<'a>(errs: impl IntoIterator<Item = &'a AppError>) -> Option<String> {
    let mut errs = errs.into_iter();
    let mut message = errs.next()?.user_message();
    let resto = errs.count();
    if resto > 0 {
        message.push_str("\n\n");
        message.push_str(&format(i18n::t("error.more"), &[("n", &resto.to_string())]));
    }
    Some(message)
}

fn show(app: &AppHandle, origin: Option<&WebviewWindow>, message: String) {
    let mut dialog = app
        .dialog()
        .message(message)
        .title("Miso")
        .kind(MessageDialogKind::Error);
    if let Some(window) = origin {
        dialog = dialog.parent(window);
    }
    dialog.show(|_| {});
}

pub fn report(app: &AppHandle, origin: Option<&WebviewWindow>, err: &AppError, detail: &str) {
    log::error!("{detail}: {err:?}");
    show(app, origin, err.user_message());
}

pub fn report_batch(app: &AppHandle, origin: Option<&WebviewWindow>, errs: &[(String, AppError)]) {
    for (detail, err) in errs {
        log::error!("{detail}: {err:?}");
    }
    if let Some(message) = batch_message(errs.iter().map(|(_, err)| err)) {
        show(app, origin, message);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn variants() -> Vec<AppError> {
        vec![
            AppError::Read {
                file: "libro.txt".into(),
                source: io::Error::new(io::ErrorKind::NotFound, "no encontrado"),
            },
            AppError::Unsupported {
                file: "libro.bin".into(),
            },
            AppError::Persist {
                source: io::Error::new(io::ErrorKind::PermissionDenied, "acceso denegado"),
            },
            AppError::Internal("detalle técnico".into()),
            AppError::Corrupt {
                file: "libro.docx".into(),
                detail: "zip inválido".into(),
            },
            AppError::Protected {
                file: "libro.pdf".into(),
            },
            AppError::Write {
                file: "copia.txt".into(),
                source: io::Error::new(io::ErrorKind::PermissionDenied, "acceso denegado"),
            },
        ]
    }

    #[test]
    fn os_message_quita_el_codigo_del_sistema() {
        let msg = os_message(&io::Error::from_raw_os_error(3));
        assert!(!msg.contains("os error"), "{msg}");
        assert!(!msg.is_empty());
    }

    #[test]
    fn os_message_sin_sufijo_queda_igual() {
        assert_eq!(os_message(&io::Error::other("algo")), "algo");
    }

    #[test]
    fn user_message_no_contiene_separadores_de_ruta() {
        for err in variants() {
            let msg = err.user_message();
            assert!(!msg.contains('/') && !msg.contains('\\'), "{msg}");
        }
    }

    #[test]
    fn user_message_de_interno_oculta_el_detalle() {
        let msg = AppError::Internal("detalle técnico".into()).user_message();
        assert_eq!(msg, "An internal error occurred.");
    }

    #[test]
    fn batch_message_vacio_es_none() {
        assert_eq!(batch_message(std::iter::empty()), None);
    }

    #[test]
    fn batch_message_con_un_error_es_su_mensaje() {
        let errs = variants();
        let msg = batch_message(errs.iter().take(1)).unwrap();
        assert_eq!(msg, errs[0].user_message());
    }

    #[test]
    fn batch_message_con_tres_errores_cuenta_los_demas() {
        let errs = variants();
        let msg = batch_message(errs.iter().take(3)).unwrap();
        assert_eq!(msg, format!("{}\n\nAnd 2 more.", errs[0].user_message()));
    }

    #[test]
    fn message_in_traduce_al_idioma_pedido() {
        let err = AppError::Unsupported { file: "libro.bin".into() };
        assert_eq!(err.message_in("es"), "«libro.bin» no es un archivo de texto.");
        let ru = err.message_in("ru");
        assert!(ru.contains("libro.bin") && !ru.contains('{'), "{ru}");
    }

    #[test]
    fn serializa_a_kind_y_message() {
        let expected = [
            ("read", "Could not read “libro.txt”: no encontrado"),
            ("unsupported", "“libro.bin” is not a text file."),
            ("persist", "Could not save the preferences: acceso denegado"),
            ("internal", "An internal error occurred."),
            ("corrupt", "Could not open “libro.docx”: the file is damaged."),
            ("protected", "Could not open “libro.pdf”: it is protected by a password or DRM."),
            ("write", "Could not save “copia.txt”: acceso denegado"),
        ];
        for (err, (kind, message)) in variants().iter().zip(expected) {
            let json = serde_json::to_value(err).unwrap();
            assert_eq!(json, serde_json::json!({ "kind": kind, "message": message }));
        }
    }
}
