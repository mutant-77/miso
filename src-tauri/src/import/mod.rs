pub mod docx;
pub mod epub;
pub mod html;
pub mod pdf;
pub mod plain;
pub mod rtf;

use std::fmt::Display;
use std::io::{Cursor, Read};
use std::path::Path;

use quick_xml::XmlVersion;
use quick_xml::events::{BytesRef, BytesStart};
use zip::ZipArchive;
use zip::result::ZipError;

use crate::error::AppError;
use crate::model::{Block, Composition, Doc, Image, Run};

const LIMITE: u64 = 256 * 1024 * 1024;

pub enum ParseError {
    Unsupported,
    Corrupt(String),
    Protected,
    Internal(String),
}

pub struct Parsed {
    pub blocks: Vec<Block>,
    pub title: Option<String>,
    pub images: Vec<Image>,
    pub composition: Composition,
    pub chapters: Vec<u32>,
}

impl From<Vec<Block>> for Parsed {
    fn from(blocks: Vec<Block>) -> Self {
        Self {
            blocks,
            title: None,
            images: Vec::new(),
            composition: Composition::Book,
            chapters: Vec::new(),
        }
    }
}

pub fn corrupt(err: impl Display) -> ParseError {
    ParseError::Corrupt(err.to_string())
}

pub fn attr(e: &BytesStart, name: &str) -> Result<Option<String>, ParseError> {
    e.try_get_attribute(name)
        .map_err(corrupt)?
        .map(|a| a.normalized_value(XmlVersion::Implicit1_0).map(|v| v.into_owned()).map_err(corrupt))
        .transpose()
}

pub fn entity(referencia: &BytesRef) -> Result<Option<char>, ParseError> {
    if referencia.is_char_ref() {
        return referencia.resolve_char_ref().map_err(corrupt);
    }
    let nombre: &str = referencia;
    Ok(match nombre {
        "amp" => Some('&'),
        "lt" => Some('<'),
        "gt" => Some('>'),
        "quot" => Some('"'),
        "apos" => Some('\''),
        _ => None,
    })
}

/// Lee una parte del zip con tope `LIMITE`; `None` si no existe.
pub fn entry(zip: &mut ZipArchive<Cursor<&[u8]>>, name: &str) -> Result<Option<Vec<u8>>, ParseError> {
    let file = match zip.by_name(name) {
        Ok(file) => file,
        Err(ZipError::FileNotFound) => return Ok(None),
        Err(err) => return Err(corrupt(err)),
    };
    let mut bytes = Vec::new();
    file.take(LIMITE + 1).read_to_end(&mut bytes).map_err(corrupt)?;
    if bytes.len() as u64 > LIMITE {
        return Err(ParseError::Corrupt(format!("«{name}» supera el tamaño permitido")));
    }
    Ok(Some(bytes))
}

pub fn file_name(path: &Path) -> String {
    path.file_name().map_or_else(
        || path.display().to_string(),
        |name| name.to_string_lossy().into_owned(),
    )
}

pub fn read(path: &Path) -> Result<Vec<u8>, AppError> {
    std::fs::read(path).map_err(|source| AppError::Read {
        file: file_name(path),
        source,
    })
}

pub fn clean(s: &str) -> String {
    s.replace(['\r', '\u{FFFC}', '\u{AD}'], "").replace('\n', " ")
}

pub fn push_run(
    runs: &mut Vec<Run>,
    raw: &str,
    bold: Option<bool>,
    italic: Option<bool>,
    underline: Option<bool>,
) {
    let texto = clean(raw);
    if texto.is_empty() {
        return;
    }
    let fusionable = runs.last_mut().filter(|last| {
        last.bold == bold && last.italic == italic && last.underline == underline
    });
    if let Some(last) = fusionable {
        last.text.push_str(&texto);
        return;
    }
    runs.push(Run { text: texto, bold, italic, underline });
}

pub fn from_bytes(file: &str, bytes: &[u8]) -> Result<Doc, AppError> {
    let parsed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| parse(file, bytes)))
        .unwrap_or_else(|_| Err(ParseError::Corrupt("pánico".into())));
    let parsed = parsed.map_err(|err| match err {
        ParseError::Unsupported => AppError::Unsupported { file: file.to_owned() },
        ParseError::Corrupt(detail) => AppError::Corrupt {
            file: file.to_owned(),
            detail,
        },
        ParseError::Protected => AppError::Protected { file: file.to_owned() },
        ParseError::Internal(detail) => AppError::Internal(detail),
    })?;
    Ok(Doc {
        title: parsed.title.unwrap_or_else(|| file.to_owned()),
        initial_char_index: 0,
        composition: parsed.composition,
        chapters: parsed.chapters,
        blocks: parsed.blocks,
        images: parsed.images,
    })
}

fn is_html(file: &str) -> bool {
    Path::new(file)
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| matches!(ext.to_ascii_lowercase().as_str(), "htm" | "html" | "xhtml"))
}

fn parse(file: &str, bytes: &[u8]) -> Result<Parsed, ParseError> {
    if bytes.starts_with(b"%PDF-") {
        return pdf::parse(bytes).map(Parsed::from);
    }
    if bytes.starts_with(b"{\\rtf") {
        return Ok(rtf::parse(bytes).into());
    }
    if bytes.starts_with(b"PK\x03\x04") {
        if epub::is_epub(bytes) {
            return epub::parse(bytes);
        }
        return docx::parse(bytes).map(Parsed::from);
    }
    if is_html(file) {
        let text = plain::decode(bytes).map_err(|_| ParseError::Unsupported)?;
        return Ok(html::parse(&text).into());
    }
    let blocks = plain::parse(bytes).map_err(|_| ParseError::Unsupported)?;
    Ok(Parsed {
        blocks,
        title: None,
        images: Vec::new(),
        composition: Composition::Text,
        chapters: Vec::new(),
    })
}

#[cfg(test)]
pub fn import(path: &Path) -> Result<Doc, AppError> {
    from_bytes(&file_name(path), &read(path)?)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{clean, epub, from_bytes, import};
    use crate::error::AppError;
    use crate::model::{Composition, Doc};

    macro_rules! fixture {
        ($name:literal) => {
            Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../tests/fixtures/", $name))
        };
    }

    fn lineas(doc: &Doc) -> Vec<String> {
        doc.blocks
            .iter()
            .map(|block| block.runs().iter().map(|run| run.text.as_str()).collect())
            .collect()
    }

    const ESPERADO: [&str; 3] = [
        "Año nuevo, señor.",
        "Café con leche: 3 € la taza.",
        "Ñandú canta en el jardín.",
    ];

    fn assert_esperado(fixture: &str, doc: &Doc) {
        assert_eq!(lineas(doc), ESPERADO, "{fixture}");
    }

    #[test]
    fn utf8_se_decodifica() {
        let doc = import(fixture!("utf8.txt")).unwrap();
        assert_esperado("utf8.txt", &doc);
    }

    #[test]
    fn utf8_con_bom_se_decodifica() {
        let doc = import(fixture!("utf8-bom.txt")).unwrap();
        assert_esperado("utf8-bom.txt", &doc);
    }

    #[test]
    fn utf16le_se_decodifica() {
        let doc = import(fixture!("utf16le.txt")).unwrap();
        assert_esperado("utf16le.txt", &doc);
    }

    #[test]
    fn utf16be_se_decodifica() {
        let doc = import(fixture!("utf16be.txt")).unwrap();
        assert_esperado("utf16be.txt", &doc);
    }

    #[test]
    fn cp1252_se_detecta_y_decodifica() {
        let doc = import(fixture!("cp1252.txt")).unwrap();
        assert_esperado("cp1252.txt", &doc);
    }

    #[test]
    fn vacio_no_tiene_bloques() {
        let doc = import(fixture!("vacio.txt")).unwrap();
        assert!(doc.blocks.is_empty());
        assert_eq!(doc.flat_len(), 0);
    }

    #[test]
    fn binario_es_unsupported() {
        let err = import(fixture!("binario.bin")).unwrap_err();
        assert!(matches!(err, AppError::Unsupported { ref file } if file == "binario.bin"));
    }

    #[test]
    fn ruta_inexistente_es_read() {
        let err = import(fixture!("no_existe.txt")).unwrap_err();
        assert!(matches!(err, AppError::Read { ref file, .. } if file == "no_existe.txt"));
    }

    #[test]
    fn unicode_normaliza_y_cuenta_k() {
        let doc = import(fixture!("unicode.txt")).unwrap();
        let texto = lineas(&doc).join("\n");
        assert!(!texto.contains('\r') && !texto.contains('\u{FFFC}'));
        // Bloques: "Hola 😀"(5+2) "中文"(2) "café xy"(4+1+1+2) ""(0) "fin"(3) ""(0)
        // = 20 unidades UTF-16 + 5 separadores (6 bloques - 1).
        assert_eq!(doc.flat_len(), 25);
    }

    #[test]
    fn clean_quita_objetos_y_une_saltos() {
        assert_eq!(clean("a\r\nb\u{FFFC}c\u{AD}d\ne"), "a bcd e");
    }

    #[test]
    fn rtf_por_contenido_se_importa_como_rtf() {
        let doc = from_bytes("a.rtf", b"{\\rtf1 hola}").unwrap();
        assert_eq!(lineas(&doc), ["hola"]);
    }

    #[test]
    fn html_por_extension_se_importa_como_html() {
        let doc = from_bytes("a.html", b"<p>hola</p>").unwrap();
        assert_eq!(lineas(&doc), ["hola"]);
    }

    #[test]
    fn txt_con_etiquetas_es_texto_plano() {
        let doc = from_bytes("a.txt", b"<html><p>hola</p></html>").unwrap();
        assert_eq!(lineas(&doc), ["<html><p>hola</p></html>"]);
    }

    #[test]
    fn zip_invalido_es_corrupt() {
        let err = from_bytes("libro.docx", b"PK\x03\x04resto").unwrap_err();
        assert!(matches!(err, AppError::Corrupt { ref file, .. } if file == "libro.docx"));
    }

    #[test]
    fn epub_de_varios_capitulos_llega_al_doc() {
        let doc = from_bytes("libro.epub", &epub::tests::tres_capitulos()).unwrap();
        assert_eq!(doc.composition, Composition::Book);
        assert_eq!(doc.chapters, [2]);
    }

    #[test]
    fn composicion_segun_formato() {
        const DOCX: &[u8] = br#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:t>hola</w:t></w:r></w:p></w:body></w:document>"#;
        let texto = from_bytes("a.txt", b"hola").unwrap();
        assert_eq!(texto.composition, Composition::Text);
        let libros = [
            from_bytes("a.html", b"<p>a</p>").unwrap(),
            from_bytes("a.rtf", b"{\\rtf1 a}").unwrap(),
            from_bytes("libro.docx", &epub::tests::zip(&[("word/document.xml", DOCX)])).unwrap(),
        ];
        for doc in libros {
            assert_eq!(doc.composition, Composition::Book);
            assert!(doc.chapters.is_empty());
        }
    }

    #[test]
    fn unicode_coincide_con_su_json() {
        let doc = import(fixture!("unicode.txt")).unwrap();
        let esperado: serde_json::Value = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../tests/fixtures/unicode.doc.json"
        )))
        .unwrap();
        assert_eq!(serde_json::to_value(&doc).unwrap(), esperado);
    }
}
