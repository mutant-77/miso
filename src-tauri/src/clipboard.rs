use crate::i18n;
use crate::import::plain;
use crate::model::{Composition, Doc};

fn doc_from_text(text: &str) -> Option<Doc> {
    if text.trim().is_empty() {
        return None;
    }
    Some(Doc {
        title: i18n::t("clipboard.title").into(),
        initial_char_index: 0,
        composition: Composition::Text,
        chapters: Vec::new(),
        blocks: plain::blocks(text),
        images: Vec::new(),
    })
}

pub fn read_doc() -> Option<Doc> {
    let mut clipboard = arboard::Clipboard::new()
        .inspect_err(|err| log::warn!("portapapeles no disponible: {err}"))
        .ok()?;
    match clipboard.get_text() {
        Ok(text) => doc_from_text(&text),
        Err(arboard::Error::ContentNotAvailable) => None,
        Err(err) => {
            log::warn!("no se pudo leer el portapapeles: {err}");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::doc_from_text;
    use crate::i18n;
    use crate::model::Composition;

    #[test]
    fn vacio_no_da_documento() {
        assert!(doc_from_text("").is_none());
    }

    #[test]
    fn solo_blancos_no_dan_documento() {
        assert!(doc_from_text("  \n\t ").is_none());
    }

    #[test]
    fn texto_da_documento_titulado() {
        let doc = doc_from_text("hola\r\nmundo").expect("documento");
        assert_eq!(doc.title, i18n::t("clipboard.title"));
        assert_eq!(doc.initial_char_index, 0);
        assert_eq!(doc.composition, Composition::Text);
        assert_eq!(doc.blocks.len(), 2);
    }
}
