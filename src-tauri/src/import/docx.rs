use std::collections::HashMap;
use std::io::Cursor;

use quick_xml::events::{BytesRef, BytesStart, Event};
use quick_xml::reader::Reader;
use zip::ZipArchive;

use crate::import::{ParseError, attr, corrupt, entity, entry, push_run};
use crate::model::{Block, Run};

// Un cuadro de texto lleva sus propios w:p dentro de un párrafo: leerlos partiría el de fuera.
const OCULTOS: [&str; 6] = ["w:del", "w:instrText", "mc:Fallback", "w:drawing", "w:pict", "w:txbxContent"];

#[derive(Clone, Copy, Default)]
enum Shape {
    #[default]
    Paragraph,
    Heading(u8),
    Item(u8),
    Quote,
}

#[derive(Clone, Copy, Default)]
struct Style {
    bold: bool,
    italic: bool,
    underline: bool,
}

#[derive(Default)]
struct Para {
    style: Shape,
    shape: Shape,
    num_id: Option<String>,
    ilvl: u32,
    marker: Option<String>,
    runs: Vec<Run>,
    split: bool,
}

enum Format {
    Bullet,
    Hidden,
    Decimal,
}

#[derive(Default)]
struct Numbering {
    abstract_of: HashMap<String, String>,
    formats: HashMap<(String, u32), String>,
}

impl Numbering {
    fn format(&self, num_id: &str, ilvl: u32) -> Format {
        let fmt = self
            .abstract_of
            .get(num_id)
            .and_then(|abs| self.formats.get(&(abs.clone(), ilvl)));
        match fmt.map(String::as_str) {
            Some("none") => Format::Hidden,
            None | Some("bullet") => Format::Bullet,
            Some(_) => Format::Decimal,
        }
    }
}

pub fn parse(bytes: &[u8]) -> Result<Vec<Block>, ParseError> {
    let mut zip = ZipArchive::new(Cursor::new(bytes)).map_err(corrupt)?;
    let Some(documento) = part(&mut zip, "word/document.xml")? else {
        return Err(ParseError::Unsupported);
    };
    let estilos = match part(&mut zip, "word/styles.xml")? {
        Some(xml) => parse_styles(&xml)?,
        None => HashMap::new(),
    };
    let numeracion = match part(&mut zip, "word/numbering.xml")? {
        Some(xml) => parse_numbering(&xml)?,
        None => Numbering::default(),
    };
    Walker::new(&estilos, &numeracion).run(&documento)
}

fn part(zip: &mut ZipArchive<Cursor<&[u8]>>, name: &str) -> Result<Option<String>, ParseError> {
    let Some(bytes) = entry(zip, name)? else {
        return Ok(None);
    };
    String::from_utf8(bytes)
        .map(Some)
        .map_err(|_| ParseError::Corrupt(format!("«{name}» no es UTF-8")))
}

fn encendido(e: &BytesStart) -> Result<bool, ParseError> {
    Ok(!matches!(attr(e, "w:val")?.as_deref(), Some("0" | "false" | "off" | "none")))
}

fn shape_of(name: &str) -> Option<Shape> {
    match name.to_lowercase().as_str() {
        "title" => Some(Shape::Heading(1)),
        "quote" | "intense quote" => Some(Shape::Quote),
        other => other
            .strip_prefix("heading ")?
            .parse::<u8>()
            .ok()
            .filter(|level| (1..=6).contains(level))
            .map(Shape::Heading),
    }
}

fn parse_styles(xml: &str) -> Result<HashMap<String, Shape>, ParseError> {
    let mut reader = Reader::from_str(xml);
    let mut map = HashMap::new();
    let mut current: Option<String> = None;
    loop {
        match reader.read_event().map_err(corrupt)? {
            Event::Start(e) if e.name().as_ref() == "w:style" => {
                current = attr(&e, "w:styleId")?;
            }
            Event::End(e) if e.name().as_ref() == "w:style" => current = None,
            Event::Start(e) | Event::Empty(e) if e.name().as_ref() == "w:name" => {
                let shape = attr(&e, "w:val")?.as_deref().and_then(shape_of);
                if let (Some(id), Some(shape)) = (&current, shape) {
                    map.insert(id.clone(), shape);
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    Ok(map)
}

fn parse_numbering(xml: &str) -> Result<Numbering, ParseError> {
    let mut reader = Reader::from_str(xml);
    let mut numbering = Numbering::default();
    let mut abstract_id: Option<String> = None;
    let mut level: Option<u32> = None;
    let mut num_id: Option<String> = None;
    loop {
        match reader.read_event().map_err(corrupt)? {
            Event::Start(e) | Event::Empty(e) => match e.name().as_ref() {
                "w:abstractNum" => abstract_id = attr(&e, "w:abstractNumId")?,
                "w:lvl" => level = attr(&e, "w:ilvl")?.and_then(|v| v.parse().ok()),
                "w:numFmt" => {
                    if let (Some(abs), Some(lvl), Some(val)) =
                        (&abstract_id, level, attr(&e, "w:val")?)
                    {
                        numbering.formats.insert((abs.clone(), lvl), val);
                    }
                }
                "w:num" => num_id = attr(&e, "w:numId")?,
                "w:abstractNumId" => {
                    if let (Some(num), Some(abs)) = (&num_id, attr(&e, "w:val")?) {
                        numbering.abstract_of.insert(num.clone(), abs);
                    }
                }
                _ => {}
            },
            Event::End(e) => match e.name().as_ref() {
                "w:abstractNum" => abstract_id = None,
                "w:lvl" => level = None,
                "w:num" => num_id = None,
                _ => {}
            },
            Event::Eof => break,
            _ => {}
        }
    }
    Ok(numbering)
}

fn flag(on: bool) -> Option<bool> {
    on.then_some(true)
}

struct Walker<'a> {
    styles: &'a HashMap<String, Shape>,
    numbering: &'a Numbering,
    out: Vec<Block>,
    para: Para,
    style: Style,
    in_run: bool,
    in_text: bool,
    hidden: u32,
    counters: HashMap<(String, u32), u32>,
    rows: Vec<Vec<String>>,
}

impl<'a> Walker<'a> {
    fn new(styles: &'a HashMap<String, Shape>, numbering: &'a Numbering) -> Self {
        Self {
            styles,
            numbering,
            out: Vec::new(),
            para: Para::default(),
            style: Style::default(),
            in_run: false,
            in_text: false,
            hidden: 0,
            counters: HashMap::new(),
            rows: Vec::new(),
        }
    }
}

impl Walker<'_> {
    fn run(mut self, xml: &str) -> Result<Vec<Block>, ParseError> {
        let mut reader = Reader::from_str(xml);
        loop {
            match reader.read_event().map_err(corrupt)? {
                Event::Start(e) => self.start(&e)?,
                Event::Empty(e) => {
                    self.start(&e)?;
                    self.end(e.name().as_ref());
                }
                Event::End(e) => self.end(e.name().as_ref()),
                Event::Text(t) => {
                    if self.in_text && self.hidden == 0 {
                        self.push_text(&t);
                    }
                }
                Event::GeneralRef(r) => self.reference(&r)?,
                Event::Eof => break,
                _ => {}
            }
        }
        Ok(self.out)
    }

    fn start(&mut self, e: &BytesStart) -> Result<(), ParseError> {
        let nombre = e.name();
        let nombre: &str = nombre.as_ref();
        if OCULTOS.contains(&nombre) {
            self.hidden += 1;
            return Ok(());
        }
        if self.hidden > 0 {
            return Ok(());
        }
        match nombre {
            "w:p" => self.para = Para::default(),
            "w:r" => {
                self.style = Style::default();
                self.in_run = true;
            }
            "w:t" => self.in_text = true,
            "w:pStyle" => {
                self.para.style = attr(e, "w:val")?
                    .and_then(|v| self.styles.get(&v).copied())
                    .unwrap_or_default();
            }
            "w:numId" => self.para.num_id = attr(e, "w:val")?,
            "w:ilvl" => {
                self.para.ilvl = attr(e, "w:val")?.and_then(|v| v.parse().ok()).unwrap_or(0);
            }
            "w:b" if self.in_run => self.style.bold = encendido(e)?,
            "w:i" if self.in_run => self.style.italic = encendido(e)?,
            "w:u" if self.in_run => self.style.underline = encendido(e)?,
            "w:tab" if self.in_run => self.push_text("\t"),
            "w:noBreakHyphen" if self.in_run => self.push_text("-"),
            "w:br" | "w:cr" if self.in_run => self.break_line(),
            "w:tr" => self.rows.push(Vec::new()),
            "w:tc" => {
                if let Some(fila) = self.rows.last_mut() {
                    fila.push(String::new());
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn end(&mut self, nombre: &str) {
        if self.hidden > 0 {
            if OCULTOS.contains(&nombre) {
                self.hidden -= 1;
            }
            return;
        }
        match nombre {
            "w:pPr" => self.list_state(),
            "w:p" => self.para_end(),
            "w:r" => self.in_run = false,
            "w:t" => self.in_text = false,
            "w:tr" => self.row_end(),
            _ => {}
        }
    }

    fn reference(&mut self, referencia: &BytesRef) -> Result<(), ParseError> {
        if !self.in_text || self.hidden > 0 {
            return Ok(());
        }
        if let Some(c) = entity(referencia)? {
            self.push_text(&c.to_string());
        }
        Ok(())
    }

    fn push_text(&mut self, raw: &str) {
        let estilo = self.style;
        push_run(
            &mut self.para.runs,
            raw,
            flag(estilo.bold),
            flag(estilo.italic),
            flag(estilo.underline),
        );
    }

    fn break_line(&mut self) {
        if !self.rows.is_empty() {
            self.push_text(" ");
            return;
        }
        let runs = std::mem::take(&mut self.para.runs);
        let marker = self.para.marker.take();
        let shape = self.para.shape;
        self.emit(shape, runs, marker);
        self.para.split = true;
    }

    fn list_state(&mut self) {
        self.para.shape = self.para.style;
        let Some(num_id) = self.para.num_id.take().filter(|id| id != "0") else {
            return;
        };
        let ilvl = self.para.ilvl;
        let count = self.counters.entry((num_id.clone(), ilvl)).or_insert(0);
        *count += 1;
        let n = *count;
        self.counters
            .retain(|(id, nivel), _| *id != num_id || *nivel <= ilvl);
        self.para.shape = Shape::Item(u8::try_from(ilvl).unwrap_or(u8::MAX));
        self.para.marker = match self.numbering.format(&num_id, ilvl) {
            Format::Bullet => Some("• ".to_owned()),
            Format::Hidden => None,
            Format::Decimal => Some(format!("{n}. ")),
        };
    }

    fn para_end(&mut self) {
        let runs = std::mem::take(&mut self.para.runs);
        let marker = self.para.marker.take();
        if !self.rows.is_empty() {
            let mut texto = marker.unwrap_or_default();
            texto.extend(runs.iter().map(|run| run.text.as_str()));
            self.cell_append(&texto);
        } else if !(self.para.split && runs.is_empty()) {
            let shape = self.para.shape;
            self.emit(shape, runs, marker);
        }
    }

    fn row_end(&mut self) {
        let Some(celdas) = self.rows.pop() else {
            return;
        };
        let fila = celdas.join("\t");
        if self.rows.is_empty() {
            self.out.push(Block::Paragraph { runs: vec![Run::plain(fila)] });
        } else {
            self.cell_append(&fila);
        }
    }

    fn cell_append(&mut self, texto: &str) {
        if texto.is_empty() {
            return;
        }
        let Some(celda) = self.rows.last_mut().and_then(|fila| fila.last_mut()) else {
            return;
        };
        if !celda.is_empty() {
            celda.push(' ');
        }
        celda.push_str(texto);
    }

    fn emit(&mut self, shape: Shape, mut runs: Vec<Run>, marker: Option<String>) {
        if let Some(marker) = marker {
            runs.insert(0, Run::plain(marker));
        }
        if runs.is_empty() {
            runs.push(Run::plain(String::new()));
        }
        self.out.push(match shape {
            Shape::Paragraph => Block::Paragraph { runs },
            Shape::Heading(level) => Block::Heading { level, runs },
            Shape::Item(depth) => Block::ListItem { depth, runs },
            Shape::Quote => Block::Quote { runs },
        });
    }
}

#[cfg(test)]
mod tests {
    use std::io::{Cursor, Write};

    use zip::CompressionMethod;
    use zip::write::{SimpleFileOptions, ZipWriter};

    use super::{ParseError, parse};
    use crate::model::Block;

    const NS: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";

    fn zip_con(partes: &[(&str, String)]) -> Vec<u8> {
        let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
        for (nombre, xml) in partes {
            let opciones = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
            zip.start_file(*nombre, opciones).unwrap();
            zip.write_all(xml.as_bytes()).unwrap();
        }
        zip.finish().unwrap().into_inner()
    }

    fn documento(cuerpo: &str) -> String {
        format!(r#"<w:document xmlns:w="{NS}"><w:body>{cuerpo}</w:body></w:document>"#)
    }

    fn doc(cuerpo: &str) -> Vec<u8> {
        zip_con(&[("word/document.xml", documento(cuerpo))])
    }

    fn estilos() -> String {
        format!(
            r#"<w:styles xmlns:w="{NS}"><w:style w:type="paragraph" w:styleId="Ttulo1"><w:name w:val="heading 1"/></w:style><w:style w:type="paragraph" w:styleId="Cita"><w:name w:val="Quote"/></w:style></w:styles>"#
        )
    }

    fn numeracion() -> String {
        format!(
            r#"<w:numbering xmlns:w="{NS}"><w:abstractNum w:abstractNumId="0"><w:lvl w:ilvl="0"><w:numFmt w:val="bullet"/></w:lvl><w:lvl w:ilvl="1"><w:numFmt w:val="decimal"/></w:lvl></w:abstractNum><w:abstractNum w:abstractNumId="1"><w:lvl w:ilvl="0"><w:numFmt w:val="decimal"/></w:lvl><w:lvl w:ilvl="1"><w:numFmt w:val="decimal"/></w:lvl></w:abstractNum><w:num w:numId="1"><w:abstractNumId w:val="0"/></w:num><w:num w:numId="2"><w:abstractNumId w:val="1"/></w:num></w:numbering>"#
        )
    }

    fn item(num: u32, ilvl: u32, texto: &str) -> String {
        format!(
            r#"<w:p><w:pPr><w:numPr><w:ilvl w:val="{ilvl}"/><w:numId w:val="{num}"/></w:numPr></w:pPr><w:r><w:t>{texto}</w:t></w:r></w:p>"#
        )
    }

    fn bloques(bytes: &[u8]) -> Vec<Block> {
        match parse(bytes) {
            Ok(blocks) => blocks,
            Err(_) => panic!("docx no válido"),
        }
    }

    fn etiqueta(block: &Block) -> String {
        match block {
            Block::Paragraph { .. } => "p".to_owned(),
            Block::Heading { level, .. } => format!("h{level}"),
            Block::ListItem { depth, .. } => format!("li{depth}"),
            Block::Quote { .. } => "cita".to_owned(),
            Block::Image { .. } => "img".to_owned(),
        }
    }

    fn afirma(bytes: &[u8], esperado: &[(&str, &str)]) {
        let reales: Vec<(String, String)> = bloques(bytes)
            .iter()
            .map(|block| {
                let texto = block.runs().iter().map(|run| run.text.as_str()).collect::<String>();
                (etiqueta(block), texto)
            })
            .collect();
        let real: Vec<(&str, &str)> = reales
            .iter()
            .map(|(tipo, texto)| (tipo.as_str(), texto.as_str()))
            .collect();
        assert_eq!(real, esperado);
    }

    fn corridas(bytes: &[u8]) -> Vec<(String, bool, bool, bool)> {
        bloques(bytes)
            .iter()
            .flat_map(Block::runs)
            .map(|run| {
                (
                    run.text.clone(),
                    run.bold == Some(true),
                    run.italic == Some(true),
                    run.underline == Some(true),
                )
            })
            .collect()
    }

    #[test]
    fn parrafos_y_estilos_de_run() {
        afirma(
            &doc(
                r#"<w:p><w:r><w:t>a </w:t></w:r><w:r><w:rPr><w:b/><w:i/><w:u w:val="single"/></w:rPr><w:t>b</w:t></w:r><w:r><w:rPr><w:b w:val="0"/></w:rPr><w:t>c</w:t></w:r></w:p><w:p/>"#,
            ),
            &[("p", "a bc"), ("p", "")],
        );
    }

    #[test]
    fn estilos_de_run_se_leen_por_run() {
        let real = corridas(&doc(
            r#"<w:p><w:r><w:t>a </w:t></w:r><w:r><w:rPr><w:b/><w:i/><w:u/></w:rPr><w:t>b</w:t></w:r><w:r><w:rPr><w:b w:val="0"/></w:rPr><w:t>c</w:t></w:r></w:p>"#,
        ));
        let esperado = [
            ("a ", false, false, false),
            ("b", true, true, true),
            ("c", false, false, false),
        ];
        let refs: Vec<(&str, bool, bool, bool)> =
            real.iter().map(|(t, b, i, u)| (t.as_str(), *b, *i, *u)).collect();
        assert_eq!(refs, esperado);
    }

    #[test]
    fn entidades_se_decodifican() {
        afirma(&doc("<w:p><w:r><w:t>a &amp; b&#x41;</w:t></w:r></w:p>"), &[("p", "a & bA")]);
    }

    #[test]
    fn titulo_por_nombre_de_estilo() {
        let cuerpo = r#"<w:p><w:pPr><w:pStyle w:val="Ttulo1"/></w:pPr><w:r><w:t>Cap</w:t></w:r></w:p><w:p><w:pPr><w:pStyle w:val="Cita"/></w:pPr><w:r><w:t>q</w:t></w:r></w:p>"#;
        afirma(
            &zip_con(&[("word/document.xml", documento(cuerpo)), ("word/styles.xml", estilos())]),
            &[("h1", "Cap"), ("cita", "q")],
        );
    }

    #[test]
    fn vineta_de_lista() {
        let cuerpo = format!("{}{}", item(1, 0, "uno"), item(1, 0, "dos"));
        afirma(
            &zip_con(&[("word/document.xml", documento(&cuerpo)), ("word/numbering.xml", numeracion())]),
            &[("li0", "• uno"), ("li0", "• dos")],
        );
    }

    #[test]
    fn numerada_de_dos_niveles_reinicia_el_contador() {
        let cuerpo = [
            item(2, 0, "a"),
            item(2, 1, "sub"),
            item(2, 1, "sub2"),
            item(2, 0, "b"),
            item(2, 1, "sub3"),
        ]
        .concat();
        afirma(
            &zip_con(&[("word/document.xml", documento(&cuerpo)), ("word/numbering.xml", numeracion())]),
            &[
                ("li0", "1. a"),
                ("li1", "1. sub"),
                ("li1", "2. sub2"),
                ("li0", "2. b"),
                ("li1", "1. sub3"),
            ],
        );
    }

    #[test]
    fn num_id_sin_numbering_es_vineta() {
        afirma(&doc(&item(9, 0, "x")), &[("li0", "• x")]);
    }

    #[test]
    fn br_dentro_de_item_no_repite_marcador() {
        let cuerpo = r#"<w:p><w:pPr><w:numPr><w:ilvl w:val="0"/><w:numId w:val="1"/></w:numPr></w:pPr><w:r><w:t>a</w:t><w:br/><w:t>b</w:t></w:r></w:p>"#;
        afirma(
            &zip_con(&[("word/document.xml", documento(cuerpo)), ("word/numbering.xml", numeracion())]),
            &[("li0", "• a"), ("li0", "b")],
        );
    }

    #[test]
    fn tabulador_de_run_no_es_el_de_tabs() {
        afirma(
            &doc(r#"<w:p><w:pPr><w:tabs><w:tab w:val="left" w:pos="720"/></w:tabs></w:pPr><w:r><w:t>a</w:t><w:tab/><w:t>b</w:t></w:r></w:p>"#),
            &[("p", "a\tb")],
        );
    }

    #[test]
    fn tabla_une_celdas_y_parrafos() {
        afirma(
            &doc(
                "<w:tbl><w:tr><w:tc><w:p><w:r><w:t>a</w:t></w:r></w:p><w:p><w:r><w:t>b</w:t></w:r></w:p></w:tc><w:tc><w:p><w:r><w:t>c</w:t></w:r></w:p></w:tc></w:tr></w:tbl>",
            ),
            &[("p", "a b\tc")],
        );
    }

    #[test]
    fn del_y_instruccion_se_omiten() {
        afirma(
            &doc(
                r#"<w:p><w:del w:id="1"><w:r><w:delText>x</w:delText></w:r></w:del><w:r><w:instrText>PAGE</w:instrText></w:r><w:r><w:t>y</w:t></w:r></w:p>"#,
            ),
            &[("p", "y")],
        );
    }

    #[test]
    fn cuadro_de_texto_no_parte_el_parrafo() {
        afirma(
            &doc(
                r#"<w:p><w:r><w:t>antes </w:t></w:r><w:r><w:pict><w:txbxContent><w:p><w:r><w:t>caja</w:t></w:r></w:p></w:txbxContent></w:pict></w:r><w:r><w:t>después</w:t></w:r></w:p>"#,
            ),
            &[("p", "antes después")],
        );
    }

    #[test]
    fn saltos_se_limpian_y_no_quedan_en_runs() {
        let cuerpo = "<w:p><w:r><w:t xml:space=\"preserve\">a\nb</w:t><w:br/><w:t>c&#10;d</w:t></w:r></w:p>";
        let bytes = doc(cuerpo);
        afirma(&bytes, &[("p", "a b"), ("p", "c d")]);
        for block in bloques(&bytes) {
            for run in block.runs() {
                assert!(!run.text.contains('\n'), "{:?}", run.text);
            }
        }
    }

    #[test]
    fn zip_sin_document_es_unsupported() {
        let bytes = zip_con(&[("word/other.xml", "<x/>".to_owned())]);
        assert!(matches!(parse(&bytes), Err(ParseError::Unsupported)));
    }

    #[test]
    fn zip_truncado_es_corrupt() {
        let bytes = doc("<w:p><w:r><w:t>hola</w:t></w:r></w:p>");
        assert!(matches!(parse(&bytes[..bytes.len() / 2]), Err(ParseError::Corrupt(_))));
    }

    #[test]
    fn xml_mal_formado_es_corrupt() {
        assert!(matches!(parse(&doc("<w:p><w:r></w:p>")), Err(ParseError::Corrupt(_))));
    }
}
