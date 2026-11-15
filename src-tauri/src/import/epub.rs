use std::collections::{HashMap, HashSet};
use std::io::Cursor;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use percent_encoding::percent_decode_str;
use quick_xml::events::{BytesStart, Event};
use quick_xml::reader::Reader;
use zip::ZipArchive;

use crate::import::{ParseError, Parsed, attr, clean, corrupt, entity, entry, html, plain};
use crate::model::{Composition, Image};

const MIMES: [&str; 5] = ["image/jpeg", "image/png", "image/gif", "image/webp", "image/svg+xml"];
const CAPITULOS: [&str; 2] = ["application/xhtml+xml", "text/html"];

// Ids únicos en el proceso: la caché del navegador es compartida entre ventanas.
static SIGUIENTE: AtomicU32 = AtomicU32::new(1);

struct Item {
    id: String,
    href: String,
    media: String,
}

#[derive(Default)]
struct Opf {
    titulo: Option<String>,
    manifiesto: Vec<Item>,
    spine: Vec<String>,
}

pub fn is_epub(bytes: &[u8]) -> bool {
    ZipArchive::new(Cursor::new(bytes))
        .is_ok_and(|zip| zip.index_for_name("META-INF/container.xml").is_some())
}

pub fn parse(bytes: &[u8]) -> Result<Parsed, ParseError> {
    let mut zip = ZipArchive::new(Cursor::new(bytes)).map_err(corrupt)?;
    let ruta_opf = ruta_opf(&leer(&mut zip, "META-INF/container.xml")?)?;
    let opf = parse_opf(&leer(&mut zip, &ruta_opf)?)?;
    if opf.spine.is_empty() {
        return Err(ParseError::Corrupt("el OPF no tiene spine".into()));
    }
    let cifrados = match entry(&mut zip, "META-INF/encryption.xml")? {
        Some(bytes) => cifrados(&decodifica(&bytes, "encryption.xml")?)?,
        None => HashSet::new(),
    };
    let base = directorio(&ruta_opf);
    let media = opf
        .manifiesto
        .iter()
        .filter_map(|item| Some((resolve(&base, &item.href)?, item.media.clone())))
        .collect();
    let mut libro = Libro {
        zip: &mut zip,
        media,
        cifrados,
        cache: HashMap::new(),
        imagenes: Vec::new(),
    };
    let mut blocks = Vec::new();
    let mut chapters = Vec::new();
    for idref in &opf.spine {
        let Some(item) = opf.manifiesto.iter().find(|item| item.id == *idref) else {
            log::warn!("EPUB: el spine cita «{idref}», que no está en el manifiesto");
            continue;
        };
        if !CAPITULOS.contains(&item.media.as_str()) {
            log::warn!("EPUB: «{}» no es un capítulo XHTML", item.href);
            continue;
        }
        let Some(ruta) = resolve(&base, &item.href) else {
            log::warn!("EPUB: ruta no válida «{}»", item.href);
            continue;
        };
        if libro.cifrados.contains(&ruta) {
            return Err(ParseError::Protected);
        }
        let Some(bytes) = entry(libro.zip, &ruta)? else {
            log::warn!("EPUB: falta el capítulo «{ruta}»");
            continue;
        };
        let Ok(texto) = plain::decode(&bytes) else {
            log::warn!("EPUB: el capítulo «{ruta}» no es texto");
            continue;
        };
        let dir = directorio(&ruta);
        let mut resolver = |src: &str| libro.imagen(&dir, src);
        let nuevos = html::parse_with(recorta_cuerpo(&texto), &mut resolver);
        if nuevos.is_empty() {
            continue;
        }
        if !blocks.is_empty() {
            chapters.push(blocks.len() as u32);
        }
        blocks.extend(nuevos);
    }
    Ok(Parsed {
        blocks,
        title: opf.titulo.filter(|titulo| !titulo.is_empty()),
        images: libro.imagenes,
        composition: Composition::Book,
        chapters,
    })
}

/// Resuelve `href` contra `base_dir` dentro del zip; `None` si sale de la raíz o no es una ruta interna.
pub fn resolve(base_dir: &str, href: &str) -> Option<String> {
    let ruta = href.split(['#', '?']).next().unwrap_or_default();
    if ruta.is_empty() || ruta.split_once(':').is_some_and(|(esquema, _)| !esquema.contains('/')) {
        return None;
    }
    let decodificada = percent_decode_str(ruta).decode_utf8().ok()?;
    let mut partes: Vec<&str> = if decodificada.starts_with('/') {
        Vec::new()
    } else {
        base_dir.split('/').filter(|parte| !parte.is_empty()).collect()
    };
    for parte in decodificada.split('/') {
        match parte {
            "" | "." => {}
            ".." => {
                partes.pop()?;
            }
            otra => partes.push(otra),
        }
    }
    (!partes.is_empty()).then(|| partes.join("/"))
}

struct Libro<'a, 'b> {
    zip: &'a mut ZipArchive<Cursor<&'b [u8]>>,
    media: HashMap<String, String>,
    cifrados: HashSet<String>,
    cache: HashMap<String, Option<u32>>,
    imagenes: Vec<Image>,
}

impl Libro<'_, '_> {
    fn imagen(&mut self, dir: &str, src: &str) -> Option<u32> {
        let ruta = resolve(dir, src)?;
        if let Some(id) = self.cache.get(&ruta) {
            return *id;
        }
        let id = self.carga(&ruta);
        self.cache.insert(ruta, id);
        id
    }

    fn carga(&mut self, ruta: &str) -> Option<u32> {
        if self.cifrados.contains(ruta) {
            return None;
        }
        let mime = self.mime(ruta)?;
        let bytes = match entry(self.zip, ruta) {
            Ok(Some(bytes)) => bytes,
            Ok(None) => return None,
            Err(_) => {
                log::warn!("EPUB: imagen «{ruta}» omitida");
                return None;
            }
        };
        let id = SIGUIENTE.fetch_add(1, Ordering::Relaxed);
        self.imagenes.push(Image { id, mime, bytes: Arc::from(bytes) });
        Some(id)
    }

    fn mime(&self, ruta: &str) -> Option<&'static str> {
        match self.media.get(ruta) {
            Some(tipo) => MIMES.into_iter().find(|mime| *mime == tipo.as_str()),
            None => mime_por_extension(ruta),
        }
    }
}

fn mime_por_extension(ruta: &str) -> Option<&'static str> {
    let nombre = ruta.rsplit('/').next()?;
    let (_, extension) = nombre.rsplit_once('.')?;
    Some(match extension.to_ascii_lowercase().as_str() {
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        _ => return None,
    })
}

fn directorio(ruta: &str) -> String {
    ruta.rsplit_once('/').map_or_else(String::new, |(dir, _)| dir.to_owned())
}

/// Descarta lo anterior a `<body`: el parser HTML trata `<title/>` en `<head>` como texto abierto.
fn recorta_cuerpo(texto: &str) -> &str {
    texto.to_ascii_lowercase().find("<body").map_or(texto, |pos| &texto[pos..])
}

fn decodifica(bytes: &[u8], nombre: &str) -> Result<String, ParseError> {
    plain::decode(bytes)
        .map(|texto| texto.into_owned())
        .map_err(|_| ParseError::Corrupt(format!("«{nombre}» no es texto")))
}

fn leer(zip: &mut ZipArchive<Cursor<&[u8]>>, nombre: &str) -> Result<String, ParseError> {
    let bytes = entry(zip, nombre)?.ok_or_else(|| ParseError::Corrupt(format!("falta «{nombre}»")))?;
    decodifica(&bytes, nombre)
}

fn ruta_opf(contenedor: &str) -> Result<String, ParseError> {
    let mut reader = Reader::from_str(contenedor);
    loop {
        match reader.read_event().map_err(corrupt)? {
            Event::Start(e) | Event::Empty(e) if e.local_name().as_ref() == "rootfile" => {
                if let Some(ruta) = attr(&e, "full-path")? {
                    return resolve("", &ruta)
                        .ok_or_else(|| ParseError::Corrupt(format!("ruta del OPF no válida: {ruta}")));
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    Err(ParseError::Corrupt("container.xml sin rootfile".into()))
}

fn parse_opf(xml: &str) -> Result<Opf, ParseError> {
    let mut reader = Reader::from_str(xml);
    let mut opf = Opf::default();
    let mut en_metadatos = false;
    let mut en_titulo = false;
    let mut texto = String::new();
    loop {
        match reader.read_event().map_err(corrupt)? {
            Event::Start(e) => {
                match e.local_name().as_ref() {
                    "metadata" => en_metadatos = true,
                    "title" if en_metadatos && opf.titulo.is_none() => en_titulo = true,
                    _ => {}
                }
                recoge(&e, &mut opf)?;
            }
            Event::Empty(e) => recoge(&e, &mut opf)?,
            Event::End(e) => match e.local_name().as_ref() {
                "metadata" => en_metadatos = false,
                "title" if en_titulo => {
                    opf.titulo = Some(normaliza(&texto));
                    texto.clear();
                    en_titulo = false;
                }
                _ => {}
            },
            Event::Text(t) if en_titulo => texto.push_str(&t),
            Event::GeneralRef(r) if en_titulo => {
                if let Some(c) = entity(&r)? {
                    texto.push(c);
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    Ok(opf)
}

fn recoge(e: &BytesStart, opf: &mut Opf) -> Result<(), ParseError> {
    match e.local_name().as_ref() {
        "item" => {
            if let (Some(id), Some(href)) = (attr(e, "id")?, attr(e, "href")?) {
                let media = attr(e, "media-type")?.unwrap_or_default();
                opf.manifiesto.push(Item { id, href, media });
            }
        }
        "itemref" => {
            if let Some(idref) = attr(e, "idref")? {
                opf.spine.push(idref);
            }
        }
        _ => {}
    }
    Ok(())
}

fn normaliza(texto: &str) -> String {
    clean(texto).split_whitespace().collect::<Vec<_>>().join(" ")
}

fn cifrados(xml: &str) -> Result<HashSet<String>, ParseError> {
    let mut reader = Reader::from_str(xml);
    let mut rutas = HashSet::new();
    loop {
        match reader.read_event().map_err(corrupt)? {
            Event::Start(e) | Event::Empty(e) if e.local_name().as_ref() == "CipherReference" => {
                if let Some(uri) = attr(&e, "URI")?
                    && let Some(ruta) = resolve("", &uri)
                {
                    rutas.insert(ruta);
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    Ok(rutas)
}

#[cfg(test)]
pub(crate) mod tests {
    use std::io::{Cursor, Write};

    use zip::CompressionMethod;
    use zip::write::{SimpleFileOptions, ZipWriter};

    use super::{Parsed, is_epub, parse, resolve};
    use crate::error::AppError;
    use crate::import::{ParseError, from_bytes};
    use crate::model::{Block, Composition};

    const CONTENEDOR: &str = r#"<container xmlns="urn:oasis:names:tc:opendocument:xmlns:container" version="1.0"><rootfiles><rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/></rootfiles></container>"#;
    const PNG: &[u8] = b"\x89PNG imagen de prueba";

    pub(crate) fn zip(ficheros: &[(&str, &[u8])]) -> Vec<u8> {
        let mut escritor = ZipWriter::new(Cursor::new(Vec::new()));
        for (nombre, bytes) in ficheros {
            let opciones = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
            escritor.start_file(*nombre, opciones).unwrap();
            escritor.write_all(bytes).unwrap();
        }
        escritor.finish().unwrap().into_inner()
    }

    fn epub(ficheros: &[(&str, &[u8])]) -> Vec<u8> {
        let mut todos = vec![
            ("mimetype", &b"application/epub+zip"[..]),
            ("META-INF/container.xml", CONTENEDOR.as_bytes()),
        ];
        todos.extend_from_slice(ficheros);
        zip(&todos)
    }

    fn opf(metadatos: &str, manifiesto: &[String], spine: &str) -> String {
        let manifiesto = manifiesto.concat();
        format!(
            r#"<package xmlns="http://www.idpf.org/2007/opf" xmlns:dc="http://purl.org/dc/elements/1.1/" version="3.0"><metadata>{metadatos}</metadata><manifest>{manifiesto}</manifest><spine>{spine}</spine></package>"#
        )
    }

    fn item(id: &str, href: &str, media: &str) -> String {
        format!(r#"<item id="{id}" href="{href}" media-type="{media}"/>"#)
    }

    fn capitulo(cuerpo: &str) -> String {
        format!(
            r#"<html xmlns="http://www.w3.org/1999/xhtml"><head><title>Cap</title></head><body>{cuerpo}</body></html>"#
        )
    }

    fn ok(bytes: &[u8]) -> Parsed {
        match parse(bytes) {
            Ok(parsed) => parsed,
            Err(_) => panic!("epub no válido"),
        }
    }

    fn filas(parsed: &Parsed) -> Vec<String> {
        parsed
            .blocks
            .iter()
            .map(|block| match block {
                Block::Image { .. } => "<img>".to_owned(),
                otro => otro.runs().iter().map(|run| run.text.as_str()).collect(),
            })
            .collect()
    }

    fn ids(parsed: &Parsed) -> Vec<u32> {
        parsed
            .blocks
            .iter()
            .filter_map(|block| match block {
                Block::Image { id } => Some(*id),
                _ => None,
            })
            .collect()
    }

    fn corrupto(bytes: &[u8]) -> bool {
        matches!(parse(bytes), Err(ParseError::Corrupt(_)))
    }

    pub(crate) fn tres_capitulos() -> Vec<u8> {
        let manifiesto = [
            item("c1", "Text/c1.xhtml", "application/xhtml+xml"),
            item("c2", "Text/c2.xhtml", "application/xhtml+xml"),
            item("c3", "Text/c3.xhtml", "application/xhtml+xml"),
        ];
        let opf = opf(
            "",
            &manifiesto,
            r#"<itemref idref="c1"/><itemref idref="c2"/><itemref idref="c3"/>"#,
        );
        epub(&[
            ("OEBPS/content.opf", opf.as_bytes()),
            ("OEBPS/Text/c1.xhtml", capitulo("<p>uno</p><p>dos</p>").as_bytes()),
            ("OEBPS/Text/c2.xhtml", capitulo("").as_bytes()),
            ("OEBPS/Text/c3.xhtml", capitulo("<p>tres</p>").as_bytes()),
        ])
    }

    #[test]
    fn spine_manda_sobre_el_orden_del_manifiesto() {
        let manifiesto = [
            item("c2", "Text/c2.xhtml", "application/xhtml+xml"),
            item("c1", "Text/c1.xhtml", "application/xhtml+xml"),
        ];
        let opf = opf("", &manifiesto, r#"<itemref idref="c1"/><itemref idref="c2"/>"#);
        let bytes = epub(&[
            ("OEBPS/content.opf", opf.as_bytes()),
            ("OEBPS/Text/c1.xhtml", capitulo("<p>uno</p>").as_bytes()),
            ("OEBPS/Text/c2.xhtml", capitulo("<p>dos</p>").as_bytes()),
        ]);
        assert_eq!(filas(&ok(&bytes)), ["uno", "dos"]);
    }

    #[test]
    fn linear_no_tambien_se_lee() {
        let manifiesto = [
            item("c1", "Text/c1.xhtml", "application/xhtml+xml"),
            item("c2", "Text/c2.xhtml", "application/xhtml+xml"),
        ];
        let opf = opf("", &manifiesto, r#"<itemref idref="c1" linear="no"/><itemref idref="c2"/>"#);
        let bytes = epub(&[
            ("OEBPS/content.opf", opf.as_bytes()),
            ("OEBPS/Text/c1.xhtml", capitulo("<p>uno</p>").as_bytes()),
            ("OEBPS/Text/c2.xhtml", capitulo("<p>dos</p>").as_bytes()),
        ]);
        assert_eq!(filas(&ok(&bytes)), ["uno", "dos"]);
    }

    #[test]
    fn items_no_xhtml_e_idref_roto_se_saltan() {
        let manifiesto = [
            item("c1", "Text/c1.xhtml", "application/xhtml+xml"),
            item("css", "Text/s.css", "text/css"),
            item("falta", "Text/no.xhtml", "application/xhtml+xml"),
            item("c2", "Text/c2.xhtml", "application/xhtml+xml"),
        ];
        let spine = r#"<itemref idref="c1"/><itemref idref="css"/><itemref idref="nada"/><itemref idref="falta"/><itemref idref="c2"/>"#;
        let opf = opf("", &manifiesto, spine);
        let bytes = epub(&[
            ("OEBPS/content.opf", opf.as_bytes()),
            ("OEBPS/Text/c1.xhtml", capitulo("<p>uno</p>").as_bytes()),
            ("OEBPS/Text/s.css", b"p { color: red }"),
            ("OEBPS/Text/c2.xhtml", capitulo("<p>dos</p>").as_bytes()),
        ]);
        assert_eq!(filas(&ok(&bytes)), ["uno", "dos"]);
    }

    #[test]
    fn opf_en_la_raiz_lee_sus_capitulos() {
        let manifiesto = [item("c1", "Text/c1.xhtml", "application/xhtml+xml")];
        let opf = opf("", &manifiesto, r#"<itemref idref="c1"/>"#);
        let contenedor = CONTENEDOR.replace("OEBPS/content.opf", "content.opf");
        let bytes = zip(&[
            ("META-INF/container.xml", contenedor.as_bytes()),
            ("content.opf", opf.as_bytes()),
            ("Text/c1.xhtml", capitulo("<p>uno</p>").as_bytes()),
        ]);
        assert_eq!(filas(&ok(&bytes)), ["uno"]);
    }

    #[test]
    fn body_en_mayusculas_se_recorta() {
        let manifiesto = [item("c1", "Text/c1.xhtml", "application/xhtml+xml")];
        let opf = opf("", &manifiesto, r#"<itemref idref="c1"/>"#);
        let capitulo = "<HTML><HEAD><TITLE/></HEAD><BODY><P>hola</P></BODY></HTML>";
        let bytes = epub(&[
            ("OEBPS/content.opf", opf.as_bytes()),
            ("OEBPS/Text/c1.xhtml", capitulo.as_bytes()),
        ]);
        assert_eq!(filas(&ok(&bytes)), ["hola"]);
    }

    #[test]
    fn title_vacio_en_head_no_traga_el_capitulo() {
        let manifiesto = [item("c1", "Text/c1.xhtml", "application/xhtml+xml")];
        let opf = opf("", &manifiesto, r#"<itemref idref="c1"/>"#);
        let capitulo = "<html><head><title/></head><body><p>hola</p></body></html>";
        let bytes = epub(&[
            ("OEBPS/content.opf", opf.as_bytes()),
            ("OEBPS/Text/c1.xhtml", capitulo.as_bytes()),
        ]);
        assert_eq!(filas(&ok(&bytes)), ["hola"]);
    }

    #[test]
    fn titulo_con_entidad_y_blancos_queda_limpio() {
        let manifiesto = [item("c1", "Text/c1.xhtml", "application/xhtml+xml")];
        let metadatos = "<dc:title>\n  Ana &amp; Juan\n  Pérez </dc:title>";
        let opf = opf(metadatos, &manifiesto, r#"<itemref idref="c1"/>"#);
        let bytes = epub(&[
            ("OEBPS/content.opf", opf.as_bytes()),
            ("OEBPS/Text/c1.xhtml", capitulo("<p>uno</p>").as_bytes()),
        ]);
        assert_eq!(ok(&bytes).title.as_deref(), Some("Ana & Juan Pérez"));
    }

    #[test]
    fn sin_dc_title_el_titulo_es_el_archivo() {
        let manifiesto = [item("c1", "Text/c1.xhtml", "application/xhtml+xml")];
        let opf = opf("", &manifiesto, r#"<itemref idref="c1"/>"#);
        let bytes = epub(&[
            ("OEBPS/content.opf", opf.as_bytes()),
            ("OEBPS/Text/c1.xhtml", capitulo("<p>uno</p>").as_bytes()),
        ]);
        let doc = from_bytes("libro.epub", &bytes).unwrap();
        assert_eq!(doc.title, "libro.epub");
    }

    #[test]
    fn imagen_resuelve_con_porcentaje_y_se_comparte() {
        let manifiesto = [item("c1", "Text/c1.xhtml", "application/xhtml+xml")];
        let opf = opf("", &manifiesto, r#"<itemref idref="c1"/>"#);
        let cuerpo = r#"<p>a</p><img src="../Images/a%20b.png"/><img src="../Images/a%20b.png"/>"#;
        let bytes = epub(&[
            ("OEBPS/content.opf", opf.as_bytes()),
            ("OEBPS/Text/c1.xhtml", capitulo(cuerpo).as_bytes()),
            ("OEBPS/Images/a b.png", PNG),
        ]);
        let parsed = ok(&bytes);
        assert_eq!(filas(&parsed), ["a", "<img>", "<img>"]);
        let identificadores = ids(&parsed);
        assert_eq!(identificadores[0], identificadores[1]);
        assert_eq!(parsed.images.len(), 1);
        assert_eq!(parsed.images[0].mime, "image/png");
        assert_eq!(&*parsed.images[0].bytes, PNG);
    }

    #[test]
    fn mime_del_manifiesto_y_svg_por_extension() {
        let manifiesto = [
            item("c1", "Text/c1.xhtml", "application/xhtml+xml"),
            item("foto", "Images/foto", "image/webp"),
        ];
        let opf = opf("", &manifiesto, r#"<itemref idref="c1"/>"#);
        let cuerpo = r#"<img src="../Images/foto"/><img src="../Images/d.svg"/>"#;
        let bytes = epub(&[
            ("OEBPS/content.opf", opf.as_bytes()),
            ("OEBPS/Text/c1.xhtml", capitulo(cuerpo).as_bytes()),
            ("OEBPS/Images/foto", PNG),
            ("OEBPS/Images/d.svg", b"<svg/>"),
        ]);
        let mimes: Vec<&str> = ok(&bytes).images.iter().map(|img| img.mime).collect();
        assert_eq!(mimes, ["image/webp", "image/svg+xml"]);
    }

    #[test]
    fn imagenes_no_admitidas_o_rotas_se_omiten() {
        let manifiesto = [
            item("c1", "Text/c1.xhtml", "application/xhtml+xml"),
            item("tiff", "Images/t.tiff", "image/tiff"),
        ];
        let opf = opf("", &manifiesto, r#"<itemref idref="c1"/>"#);
        let cuerpo = concat!(
            "<p>a</p>",
            r#"<img src="http://x/a.png"/>"#,
            r#"<img src="data:image/png;base64,AAAA"/>"#,
            r#"<img src="../../../x.png"/>"#,
            r#"<img src="../Images/no.png"/>"#,
            r#"<img src="../Images/t.tiff"/>"#,
            r#"<img src="../Images/ok.png"/>"#,
        );
        let bytes = epub(&[
            ("OEBPS/content.opf", opf.as_bytes()),
            ("OEBPS/Text/c1.xhtml", capitulo(cuerpo).as_bytes()),
            ("x.png", PNG),
            ("OEBPS/Images/t.tiff", PNG),
            ("OEBPS/Images/ok.png", PNG),
        ]);
        let parsed = ok(&bytes);
        assert_eq!(filas(&parsed), ["a", "<img>"]);
        assert_eq!(parsed.images.len(), 1);
    }

    #[test]
    fn ids_distintos_entre_importaciones() {
        let manifiesto = [item("c1", "Text/c1.xhtml", "application/xhtml+xml")];
        let opf = opf("", &manifiesto, r#"<itemref idref="c1"/>"#);
        let bytes = epub(&[
            ("OEBPS/content.opf", opf.as_bytes()),
            ("OEBPS/Text/c1.xhtml", capitulo(r#"<img src="../Images/a.png"/>"#).as_bytes()),
            ("OEBPS/Images/a.png", PNG),
        ]);
        let primera = ids(&ok(&bytes));
        let segunda = ids(&ok(&bytes));
        assert_ne!(primera[0], segunda[0]);
    }

    #[test]
    fn epub_sin_mimetype_se_detecta() {
        let manifiesto = [item("c1", "Text/c1.xhtml", "application/xhtml+xml")];
        let opf = opf("", &manifiesto, r#"<itemref idref="c1"/>"#);
        let bytes = zip(&[
            ("META-INF/container.xml", CONTENEDOR.as_bytes()),
            ("OEBPS/content.opf", opf.as_bytes()),
            ("OEBPS/Text/c1.xhtml", capitulo("<p>uno</p>").as_bytes()),
        ]);
        assert!(is_epub(&bytes));
        assert_eq!(filas(&ok(&bytes)), ["uno"]);
    }

    #[test]
    fn zip_con_word_document_sigue_siendo_docx() {
        let documento = r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:t>hola</w:t></w:r></w:p></w:body></w:document>"#;
        let bytes = zip(&[("word/document.xml", documento.as_bytes())]);
        assert!(!is_epub(&bytes));
        let doc = from_bytes("libro.docx", &bytes).unwrap();
        assert_eq!(doc.blocks[0].runs()[0].text, "hola");
    }

    #[test]
    fn zip_sin_epub_ni_docx_es_unsupported() {
        let bytes = zip(&[("otro.txt", b"x")]);
        assert!(matches!(from_bytes("libro.zip", &bytes), Err(AppError::Unsupported { .. })));
    }

    #[test]
    fn capitulo_cifrado_es_protected() {
        let manifiesto = [item("c1", "Text/c1.xhtml", "application/xhtml+xml")];
        let opf = opf("", &manifiesto, r#"<itemref idref="c1"/>"#);
        let cifrado = r#"<encryption xmlns="urn:oasis:names:tc:opendocument:xmlns:container" xmlns:enc="http://www.w3.org/2001/04/xmlenc#"><enc:EncryptedData><enc:CipherData><enc:CipherReference URI="OEBPS/Text/c1.xhtml"/></enc:CipherData></enc:EncryptedData></encryption>"#;
        let bytes = epub(&[
            ("OEBPS/content.opf", opf.as_bytes()),
            ("OEBPS/Text/c1.xhtml", capitulo("<p>uno</p>").as_bytes()),
            ("META-INF/encryption.xml", cifrado.as_bytes()),
        ]);
        assert!(matches!(from_bytes("libro.epub", &bytes), Err(AppError::Protected { ref file }) if file == "libro.epub"));
    }

    #[test]
    fn encryption_solo_con_fuente_abre_el_libro() {
        let manifiesto = [item("c1", "Text/c1.xhtml", "application/xhtml+xml")];
        let opf = opf("", &manifiesto, r#"<itemref idref="c1"/>"#);
        let cifrado = r#"<encryption xmlns="urn:oasis:names:tc:opendocument:xmlns:container" xmlns:enc="http://www.w3.org/2001/04/xmlenc#"><enc:EncryptedData><enc:CipherData><enc:CipherReference URI="OEBPS/Fonts/f.otf"/></enc:CipherData></enc:EncryptedData></encryption>"#;
        let bytes = epub(&[
            ("OEBPS/content.opf", opf.as_bytes()),
            ("OEBPS/Text/c1.xhtml", capitulo("<p>uno</p>").as_bytes()),
            ("OEBPS/Fonts/f.otf", b"fuente"),
            ("META-INF/encryption.xml", cifrado.as_bytes()),
        ]);
        assert_eq!(filas(&ok(&bytes)), ["uno"]);
    }

    #[test]
    fn sin_rootfile_es_corrupt() {
        let bytes = zip(&[
            ("META-INF/container.xml", b"<container><rootfiles/></container>"),
        ]);
        assert!(corrupto(&bytes));
    }

    #[test]
    fn opf_inexistente_es_corrupt() {
        let bytes = epub(&[]);
        assert!(corrupto(&bytes));
    }

    #[test]
    fn opf_mal_formado_es_corrupt() {
        let bytes = epub(&[
            ("OEBPS/content.opf", b"<package><metadata></package>"),
        ]);
        assert!(corrupto(&bytes));
    }

    #[test]
    fn spine_vacio_es_corrupt() {
        let manifiesto = [item("c1", "Text/c1.xhtml", "application/xhtml+xml")];
        let opf = opf("", &manifiesto, "");
        let bytes = epub(&[
            ("OEBPS/content.opf", opf.as_bytes()),
            ("OEBPS/Text/c1.xhtml", capitulo("<p>uno</p>").as_bytes()),
        ]);
        assert!(corrupto(&bytes));
    }

    #[test]
    fn epub_truncado_es_corrupt() {
        let manifiesto = [item("c1", "Text/c1.xhtml", "application/xhtml+xml")];
        let opf = opf("", &manifiesto, r#"<itemref idref="c1"/>"#);
        let bytes = epub(&[
            ("OEBPS/content.opf", opf.as_bytes()),
            ("OEBPS/Text/c1.xhtml", capitulo("<p>uno</p>").as_bytes()),
        ]);
        assert!(matches!(from_bytes("libro.epub", &bytes[..bytes.len() / 2]), Err(AppError::Corrupt { .. })));
    }

    #[test]
    fn resolve_normaliza_rutas() {
        assert_eq!(resolve("OEBPS/Text", "../Images/a%20b.png").as_deref(), Some("OEBPS/Images/a b.png"));
        assert_eq!(resolve("", "./a/./b/../c.png").as_deref(), Some("a/c.png"));
        assert_eq!(resolve("OEBPS", "/raiz.png").as_deref(), Some("raiz.png"));
        assert_eq!(resolve("OEBPS", "c.xhtml#cap?x").as_deref(), Some("OEBPS/c.xhtml"));
    }

    #[test]
    fn resolve_rechaza_esquemas_salidas_y_vacios() {
        for href in ["http://x/a.png", "data:image/png;base64,AA", "file:///c.png", "../../../x.png", "", "#solo", "%FF.png"] {
            assert_eq!(resolve("OEBPS/Text", href), None, "{href}");
        }
    }

    #[test]
    fn capitulo_vacio_no_cuenta_y_los_demas_si() {
        let parsed = ok(&tres_capitulos());
        assert_eq!(filas(&parsed), ["uno", "dos", "tres"]);
        assert_eq!(parsed.chapters, [2]);
        assert_eq!(parsed.composition, Composition::Book);
    }

    #[test]
    fn un_solo_capitulo_no_marca_ninguno() {
        let manifiesto = [item("c1", "Text/c1.xhtml", "application/xhtml+xml")];
        let opf = opf("", &manifiesto, r#"<itemref idref="c1"/>"#);
        let bytes = epub(&[
            ("OEBPS/content.opf", opf.as_bytes()),
            ("OEBPS/Text/c1.xhtml", capitulo("<p>uno</p>").as_bytes()),
        ]);
        let parsed = ok(&bytes);
        assert!(parsed.chapters.is_empty());
        assert_eq!(parsed.composition, Composition::Book);
    }

    #[test]
    fn capitulo_que_solo_da_imagen_cuenta() {
        let manifiesto = [
            item("c1", "Text/c1.xhtml", "application/xhtml+xml"),
            item("c2", "Text/c2.xhtml", "application/xhtml+xml"),
            item("c3", "Text/c3.xhtml", "application/xhtml+xml"),
        ];
        let opf = opf(
            "",
            &manifiesto,
            r#"<itemref idref="c1"/><itemref idref="c2"/><itemref idref="c3"/>"#,
        );
        let bytes = epub(&[
            ("OEBPS/content.opf", opf.as_bytes()),
            ("OEBPS/Text/c1.xhtml", capitulo("<p>uno</p>").as_bytes()),
            ("OEBPS/Text/c2.xhtml", capitulo(r#"<img src="../Images/a.png"/>"#).as_bytes()),
            ("OEBPS/Text/c3.xhtml", capitulo("<p>tres</p>").as_bytes()),
            ("OEBPS/Images/a.png", PNG),
        ]);
        let parsed = ok(&bytes);
        assert_eq!(filas(&parsed), ["uno", "<img>", "tres"]);
        assert_eq!(parsed.chapters, [1, 2]);
    }
}
