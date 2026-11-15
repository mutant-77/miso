use scraper::{ElementRef, Html, Node};

use crate::import::clean;
use crate::model::{Block, Run};

const IGNORADOS: &[&str] = &["head", "script", "style", "template", "noscript", "math"];

const CONTENEDORES: &[&str] = &[
    "div", "section", "article", "header", "footer", "main", "nav", "aside", "figure", "figcaption",
    "address", "dd", "dt", "dl", "table", "tbody", "thead", "body", "html", "pre", "hr", "form",
    "fieldset",
];

#[derive(Clone, Copy)]
enum Kind {
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

struct Open {
    kind: Kind,
    runs: Vec<Run>,
    after_space: bool,
    marker: Option<String>,
}

struct Walker<'a> {
    out: Vec<Block>,
    open: Open,
    style: Style,
    quote: bool,
    row: bool,
    lists: Vec<(bool, u32)>,
    resolver: &'a mut dyn FnMut(&str) -> Option<u32>,
}

pub fn parse(text: &str) -> Vec<Block> {
    parse_with(text, &mut |_| None)
}

pub fn parse_with(text: &str, resolver: &mut dyn FnMut(&str) -> Option<u32>) -> Vec<Block> {
    let html = Html::parse_document(text);
    let mut walker = Walker::new(resolver);
    walker.element(html.root_element());
    walker.flush();
    walker.out
}

impl<'a> Walker<'a> {
    fn new(resolver: &'a mut dyn FnMut(&str) -> Option<u32>) -> Self {
        Self {
            out: Vec::new(),
            open: Open {
                kind: Kind::Paragraph,
                runs: Vec::new(),
                after_space: true,
                marker: None,
            },
            style: Style::default(),
            quote: false,
            row: false,
            lists: Vec::new(),
            resolver,
        }
    }

    fn children(&mut self, el: ElementRef) {
        for child in el.children() {
            match child.value() {
                Node::Text(text) => self.text(text),
                Node::Element(_) => {
                    if let Some(inner) = ElementRef::wrap(child) {
                        self.element(inner);
                    }
                }
                _ => {}
            }
        }
    }

    fn element(&mut self, el: ElementRef) {
        let elem = el.value();
        let name = elem.name();
        if IGNORADOS.contains(&name) || elem.attr("hidden").is_some() {
            return;
        }
        // Dentro de una fila todo es texto de su celda: un bloque ahí partiría la fila.
        let breaks = matches!(name, "p" | "li" | "br" | "tr") || heading_level(name).is_some();
        if self.row && (breaks || CONTENEDORES.contains(&name)) {
            self.text(" ");
            self.children(el);
            self.text(" ");
            return;
        }
        if let Some(level) = heading_level(name) {
            return self.block(el, Kind::Heading(level));
        }
        match name {
            "p" => {
                let kind = match self.open.kind {
                    Kind::Item(_) => self.open.kind,
                    _ => self.paragraph_kind(),
                };
                self.block(el, kind);
            }
            "tr" => {
                let kind = self.paragraph_kind();
                self.row = true;
                self.block(el, kind);
                self.row = false;
            }
            "li" => self.item(el),
            "td" | "th" => {
                if cell_index(el) > 0 {
                    self.separator();
                }
                self.children(el);
            }
            "br" => self.flush(),
            "img" => {
                if let Some(src) = el.value().attr("src") {
                    self.image(src);
                }
            }
            "svg" => self.svg(el),
            "b" | "strong" => self.styled(el, |s| s.bold = true),
            "i" | "em" | "cite" | "var" => self.styled(el, |s| s.italic = true),
            "u" | "ins" => self.styled(el, |s| s.underline = true),
            "blockquote" => self.quote(el),
            "ul" => self.list(el, false),
            "ol" => self.list(el, true),
            _ if CONTENEDORES.contains(&name) => {
                self.flush();
                self.children(el);
                self.flush();
            }
            _ => self.children(el),
        }
    }

    fn svg(&mut self, el: ElementRef) {
        for inner in el.descendent_elements() {
            let elem = inner.value();
            if elem.name() == "image"
                && let Some((_, href)) = elem.attrs().find(|(nombre, _)| *nombre == "href")
            {
                self.image(href);
            }
        }
    }

    fn image(&mut self, src: &str) {
        if self.row {
            return;
        }
        let Some(id) = (self.resolver)(src) else {
            return;
        };
        self.flush();
        self.out.push(Block::Image { id });
    }

    fn paragraph_kind(&self) -> Kind {
        if self.quote { Kind::Quote } else { Kind::Paragraph }
    }

    fn block(&mut self, el: ElementRef, kind: Kind) {
        self.flush();
        let outer = std::mem::replace(&mut self.open.kind, kind);
        self.children(el);
        self.flush();
        self.open.kind = outer;
    }

    fn item(&mut self, el: ElementRef) {
        let depth = u8::try_from(self.lists.len().saturating_sub(1)).unwrap_or(u8::MAX);
        let marker = self.marker();
        self.flush();
        let outer_kind = std::mem::replace(&mut self.open.kind, Kind::Item(depth));
        let outer_marker = self.open.marker.replace(marker);
        self.children(el);
        self.flush();
        self.open.marker = outer_marker;
        self.open.kind = outer_kind;
    }

    fn marker(&mut self) -> String {
        match self.lists.last_mut() {
            Some((true, count)) => {
                *count += 1;
                format!("{count}. ")
            }
            _ => "• ".to_owned(),
        }
    }

    fn list(&mut self, el: ElementRef, ordered: bool) {
        self.flush();
        self.lists.push((ordered, 0));
        self.children(el);
        self.flush();
        self.lists.pop();
    }

    fn quote(&mut self, el: ElementRef) {
        self.flush();
        let outer_kind = std::mem::replace(&mut self.open.kind, Kind::Quote);
        let outer_quote = std::mem::replace(&mut self.quote, true);
        self.children(el);
        self.flush();
        self.open.kind = outer_kind;
        self.quote = outer_quote;
    }

    fn styled(&mut self, el: ElementRef, change: fn(&mut Style)) {
        let outer = self.style;
        change(&mut self.style);
        self.children(el);
        self.style = outer;
    }

    fn separator(&mut self) {
        trim_end(&mut self.open.runs);
        self.open.runs.push(Run::plain("\t".to_owned()));
        self.open.after_space = true;
    }

    fn text(&mut self, raw: &str) {
        let mut buf = String::new();
        for c in clean(raw).chars() {
            if matches!(c, ' ' | '\t' | '\x0C') {
                if !self.open.after_space {
                    buf.push(' ');
                    self.open.after_space = true;
                }
            } else {
                buf.push(c);
                self.open.after_space = false;
            }
        }
        if buf.is_empty() {
            return;
        }
        if let Some(marker) = self.open.marker.take() {
            self.open.runs.push(Run::plain(marker));
        }
        let style = self.style;
        self.open.runs.push(Run {
            text: buf,
            bold: style.bold.then_some(true),
            italic: style.italic.then_some(true),
            underline: style.underline.then_some(true),
        });
    }

    fn flush(&mut self) {
        let mut runs = std::mem::take(&mut self.open.runs);
        trim_end(&mut runs);
        self.open.after_space = true;
        if !runs.is_empty() {
            self.out.push(make_block(self.open.kind, runs));
        }
    }
}

fn cell_index(el: ElementRef) -> usize {
    el.prev_siblings()
        .filter_map(ElementRef::wrap)
        .filter(|cell| matches!(cell.value().name(), "td" | "th"))
        .count()
}

fn heading_level(name: &str) -> Option<u8> {
    name.strip_prefix('h')?
        .parse::<u8>()
        .ok()
        .filter(|level| (1..=6).contains(level))
}

fn trim_end(runs: &mut Vec<Run>) {
    while let Some(last) = runs.last_mut() {
        let len = last.text.trim_end_matches([' ', '\t']).len();
        if len == 0 {
            runs.pop();
        } else {
            last.text.truncate(len);
            break;
        }
    }
}

fn make_block(kind: Kind, runs: Vec<Run>) -> Block {
    match kind {
        Kind::Paragraph => Block::Paragraph { runs },
        Kind::Heading(level) => Block::Heading { level, runs },
        Kind::Item(depth) => Block::ListItem { depth, runs },
        Kind::Quote => Block::Quote { runs },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn etiqueta(block: &Block) -> String {
        match block {
            Block::Paragraph { .. } => "p".to_owned(),
            Block::Heading { level, .. } => format!("h{level}"),
            Block::ListItem { depth, .. } => format!("li{depth}"),
            Block::Quote { .. } => "cita".to_owned(),
            Block::Image { .. } => "img".to_owned(),
        }
    }

    fn afirma(html: &str, esperado: &[(&str, &str)]) {
        afirma_con(html, &mut |_| None, esperado);
    }

    fn afirma_con(
        html: &str,
        resolver: &mut dyn FnMut(&str) -> Option<u32>,
        esperado: &[(&str, &str)],
    ) {
        let bloques: Vec<(String, String)> = parse_with(html, resolver)
            .iter()
            .map(|block| {
                let texto = block.runs().iter().map(|run| run.text.as_str()).collect::<String>();
                (etiqueta(block), texto)
            })
            .collect();
        let real: Vec<(&str, &str)> = bloques
            .iter()
            .map(|(tipo, texto)| (tipo.as_str(), texto.as_str()))
            .collect();
        assert_eq!(real, esperado);
    }

    fn corridas(html: &str) -> Vec<(String, bool, bool, bool)> {
        parse(html)
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

    fn afirma_corridas(html: &str, esperado: &[(&str, bool, bool, bool)]) {
        let real = corridas(html);
        let refs: Vec<(&str, bool, bool, bool)> = real
            .iter()
            .map(|(texto, b, i, u)| (texto.as_str(), *b, *i, *u))
            .collect();
        assert_eq!(refs, esperado);
    }

    #[test]
    fn parrafos_y_titulos() {
        afirma(
            "<h1>Título</h1><h3>Sub</h3><p>cuerpo</p>",
            &[("h1", "Título"), ("h3", "Sub"), ("p", "cuerpo")],
        );
    }

    #[test]
    fn colapsa_blancos_y_descarta_vacios() {
        afirma("<p></p><p>  dos  \n tres </p><p> </p>", &[("p", "dos tres")]);
    }

    #[test]
    fn parrafos_dentro_de_un_elemento_de_lista_siguen_en_la_lista() {
        afirma(
            "<ul><li><p>a</p><p>b</p></li></ul>",
            &[("li0", "• a"), ("li0", "b")],
        );
    }

    #[test]
    fn parrafos_dentro_de_una_celda_no_parten_la_fila() {
        afirma(
            "<table><tr><td><p>x</p><p>y</p></td><td>z<br>w</td></tr></table>",
            &[("p", "x y\tz w")],
        );
    }

    #[test]
    fn texto_suelto_forma_parrafos_anonimos() {
        afirma(
            "<div>suelto<p>a</p>más</div>",
            &[("p", "suelto"), ("p", "a"), ("p", "más")],
        );
    }

    #[test]
    fn listas_anidadas_con_marcadores() {
        afirma(
            "<ul><li>a<ol><li>b</li><li>c</li></ol></li></ul>",
            &[("li0", "• a"), ("li1", "1. b"), ("li1", "2. c")],
        );
    }

    #[test]
    fn ol_numera_desde_uno() {
        afirma("<ol><li>x</li><li>y</li></ol>", &[("li0", "1. x"), ("li0", "2. y")]);
    }

    #[test]
    fn br_parte_el_item_sin_marcador() {
        afirma("<ul><li>a<br>b</li></ul>", &[("li0", "• a"), ("li0", "b")]);
    }

    #[test]
    fn negrita_dentro_de_cursiva() {
        afirma_corridas(
            "<i>a <b>b</b> c</i>",
            &[("a ", false, true, false), ("b", true, true, false), (" c", false, true, false)],
        );
    }

    #[test]
    fn colapso_entre_runs_de_distinto_estilo() {
        afirma_corridas(
            "<p>a <b>b</b></p>",
            &[("a ", false, false, false), ("b", true, false, false)],
        );
    }

    #[test]
    fn tabla_une_celdas_con_tabulador() {
        afirma("<table><tr><td>x</td><td>y</td></tr></table>", &[("p", "x\ty")]);
        afirma(
            "<table>\n<tr>\n<td>x</td>\n<td>y </td>\n</tr>\n</table>",
            &[("p", "x\ty")],
        );
    }

    #[test]
    fn ignora_script_style_y_hidden() {
        afirma(
            "<p>a</p><script>x</script><style>y</style><p hidden>o</p><p>b</p>",
            &[("p", "a"), ("p", "b")],
        );
    }

    #[test]
    fn decodifica_entidades_y_conserva_nbsp() {
        afirma("<p>a &amp; b&nbsp;c</p>", &[("p", "a & b\u{a0}c")]);
    }

    #[test]
    fn ningun_run_tiene_saltos() {
        let html = "<p>a\n b<br>\nc\r\nd</p>\n<h2>\ne\n</h2><ul>\n<li>\nf<br/>g</li>\n</ul><blockquote>\nh\n</blockquote>";
        let bloques = parse(html);
        assert!(!bloques.is_empty());
        for block in &bloques {
            for run in block.runs() {
                assert!(!run.text.contains(['\n', '\r']), "{:?}", run.text);
            }
        }
    }

    #[test]
    fn quita_objetos_y_guion_blando() {
        afirma("<p>a\u{FFFC}b\u{AD}c</p>", &[("p", "abc")]);
    }

    #[test]
    fn blockquote_con_p_es_cita() {
        afirma(
            "<blockquote><p>cita</p>suelta</blockquote>",
            &[("cita", "cita"), ("cita", "suelta")],
        );
    }

    #[test]
    fn img_parte_el_parrafo_con_imagen() {
        afirma_con(
            "<p>a<img src=\"x\">b</p>",
            &mut |src| (src == "x").then_some(7),
            &[("p", "a"), ("img", ""), ("p", "b")],
        );
    }

    #[test]
    fn img_sin_resolutor_no_parte_el_parrafo() {
        afirma("<p>a<img src=\"x\">b</p>", &[("p", "ab")]);
    }

    #[test]
    fn img_dentro_de_item_no_repite_el_marcador() {
        afirma_con(
            "<ul><li>a<img src=\"x\">b</li></ul>",
            &mut |_| Some(1),
            &[("li0", "• a"), ("img", ""), ("li0", "b")],
        );
    }

    #[test]
    fn img_dentro_de_fila_no_parte_la_fila() {
        afirma_con(
            "<table><tr><td>x</td><td><img src=\"x\"></td><td>y</td></tr></table>",
            &mut |_| Some(1),
            &[("p", "x\ty")],
        );
    }

    #[test]
    fn svg_emite_su_image_con_href_o_xlink() {
        for html in [
            r#"<svg><image xlink:href="c.jpg"/></svg>"#,
            r#"<svg><image href="c.jpg"/></svg>"#,
        ] {
            let bloques = parse_with(html, &mut |src| (src == "c.jpg").then_some(3));
            assert!(matches!(bloques.as_slice(), [Block::Image { id: 3 }]), "{html}");
        }
    }
}
