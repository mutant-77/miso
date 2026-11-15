use encoding_rs::{Encoding, WINDOWS_1252};

use super::{clean, push_run};
use crate::model::{Block, Run};

const REEMPLAZO: char = '\u{FFFD}';

// Destinos cuyo grupo entero se omite; `header*` y `footer*` se miran por prefijo.
const DESTINOS: [&str; 11] = [
    "fonttbl",
    "colortbl",
    "stylesheet",
    "info",
    "pict",
    "object",
    "footnote",
    "annotation",
    "shp",
    "nonshppict",
    "fldinst",
];

#[derive(Clone, Copy)]
struct State {
    bold: bool,
    italic: bool,
    underline: bool,
    hidden: bool,
    skip: bool,
    fresh: bool,
    intbl: bool,
    in_listtext: bool,
    outline: Option<u8>,
    ilvl: u8,
    uc: usize,
}

impl Default for State {
    fn default() -> Self {
        Self {
            bold: false,
            italic: false,
            underline: false,
            hidden: false,
            skip: false,
            fresh: false,
            intbl: false,
            in_listtext: false,
            outline: None,
            ilvl: 0,
            uc: 1,
        }
    }
}

struct Parser {
    stack: Vec<State>,
    enc: &'static Encoding,
    pending: Vec<u8>,
    skip_units: usize,
    high: Option<u32>,
    runs: Vec<Run>,
    marker: Option<String>,
    item: bool,
    listtext: String,
    split: bool,
    cells: Vec<String>,
    out: Vec<Block>,
}

pub fn parse(bytes: &[u8]) -> Vec<Block> {
    let mut parser = Parser::new();
    let mut i = 0;
    while let Some(&b) = bytes.get(i) {
        i = match b {
            b'{' => {
                parser.open();
                i + 1
            }
            b'}' => {
                parser.close();
                i + 1
            }
            b'\\' => parser.escape(bytes, i + 1),
            b'\r' | b'\n' => i + 1,
            _ => {
                parser.text_byte(b);
                i + 1
            }
        };
    }
    parser.finish()
}

impl Parser {
    fn new() -> Self {
        Self {
            stack: vec![State::default()],
            enc: WINDOWS_1252,
            pending: Vec::new(),
            skip_units: 0,
            high: None,
            runs: Vec::new(),
            marker: None,
            item: false,
            listtext: String::new(),
            split: false,
            cells: Vec::new(),
            out: Vec::new(),
        }
    }

    fn state(&mut self) -> &mut State {
        if self.stack.is_empty() {
            self.stack.push(State::default());
        }
        let last = self.stack.len() - 1;
        &mut self.stack[last]
    }

    fn open(&mut self) {
        self.flush();
        self.skip_units = 0;
        self.state().fresh = false;
        let mut group = *self.state();
        group.fresh = true;
        self.stack.push(group);
    }

    fn close(&mut self) {
        self.flush();
        self.skip_units = 0;
        if self.stack.len() == 1 {
            return;
        }
        let closed = self.stack.pop().unwrap_or_default();
        if closed.in_listtext && !self.state().in_listtext {
            self.finish_marker();
        }
    }

    fn escape(&mut self, bytes: &[u8], j: usize) -> usize {
        let Some(&c) = bytes.get(j) else {
            return j;
        };
        match c {
            b'\'' => match hex_byte(bytes, j + 1) {
                Some(valor) => {
                    self.text_byte(valor);
                    j + 3
                }
                None => j + 1,
            },
            b'\\' | b'{' | b'}' => {
                self.text_byte(c);
                j + 1
            }
            b'*' => {
                self.inicio_control();
                self.state().skip = true;
                j + 1
            }
            b'~' => {
                self.inicio_control();
                self.emit_text("\u{a0}");
                j + 1
            }
            b'_' => {
                self.inicio_control();
                self.emit_text("-");
                j + 1
            }
            b'-' => {
                self.inicio_control();
                j + 1
            }
            b'\r' | b'\n' => {
                self.inicio_control();
                self.nombrado("par", None, false);
                j + 1
            }
            _ if c.is_ascii_alphabetic() => self.word(bytes, j),
            _ => {
                self.inicio_control();
                j + 1
            }
        }
    }

    fn word(&mut self, bytes: &[u8], j: usize) -> usize {
        let fin = j + bytes[j..].iter().take_while(|b| b.is_ascii_alphabetic()).count();
        let nombre = std::str::from_utf8(&bytes[j..fin]).unwrap_or("");
        let (param, mut k) = number(bytes, fin);
        if bytes.get(k) == Some(&b' ') {
            k += 1;
        }
        let fresh = self.inicio_control();
        if nombre == "bin" {
            let n = usize::try_from(param.unwrap_or(0)).unwrap_or(0);
            return k.saturating_add(n).min(bytes.len());
        }
        self.nombrado(nombre, param, fresh);
        k
    }

    fn nombrado(&mut self, nombre: &str, param: Option<i32>, fresh: bool) {
        if fresh && self.destino(nombre) {
            return;
        }
        if self.state().skip {
            return;
        }
        let on = param.unwrap_or(1) != 0;
        match nombre {
            "par" | "sect" | "page" => self.par(),
            "line" => self.line(),
            "tab" => self.emit_text("\t"),
            "cell" => self.close_cell(),
            "row" => self.row_end(),
            "pard" => {
                let st = self.state();
                st.intbl = false;
                st.outline = None;
                st.ilvl = 0;
            }
            "plain" => {
                let st = self.state();
                st.bold = false;
                st.italic = false;
                st.underline = false;
                st.hidden = false;
            }
            "b" => self.state().bold = on,
            "i" => self.state().italic = on,
            "ul" => self.state().underline = on,
            "ulnone" => self.state().underline = false,
            "v" => self.state().hidden = on,
            "intbl" => self.state().intbl = true,
            "outlinelevel" => {
                self.state().outline = param
                    .and_then(|n| u8::try_from(n).ok())
                    .filter(|n| *n <= 5);
            }
            "ilvl" => {
                self.state().ilvl =
                    u8::try_from(param.unwrap_or(0).clamp(0, 255)).unwrap_or_default();
            }
            "uc" => self.state().uc = usize::try_from(param.unwrap_or(1)).unwrap_or(0),
            "u" => {
                if let Some(n) = param {
                    self.unicode(n);
                }
            }
            "ansicpg" => {
                self.enc = param
                    .and_then(|pagina| u16::try_from(pagina).ok())
                    .and_then(codepage::to_encoding)
                    .unwrap_or(WINDOWS_1252);
            }
            _ => {
                if let Some(texto) = simbolo(nombre) {
                    self.emit_text(texto);
                }
            }
        }
    }

    fn destino(&mut self, nombre: &str) -> bool {
        if matches!(nombre, "listtext" | "pntext") {
            self.state().in_listtext = true;
            self.listtext.clear();
        } else if DESTINOS.contains(&nombre)
            || nombre.starts_with("header")
            || nombre.starts_with("footer")
        {
            self.state().skip = true;
        } else {
            return false;
        }
        true
    }

    fn text_byte(&mut self, valor: u8) {
        self.state().fresh = false;
        if self.skip_units > 0 {
            self.skip_units -= 1;
        } else if !self.state().skip {
            self.pending.push(valor);
        }
    }

    fn inicio_control(&mut self) -> bool {
        self.flush();
        self.skip_units = 0;
        std::mem::replace(&mut self.state().fresh, false)
    }

    fn flush(&mut self) {
        if self.pending.is_empty() {
            return;
        }
        let texto = self.enc.decode_without_bom_handling(&self.pending).0.into_owned();
        self.pending.clear();
        self.emit_text(&texto);
    }

    fn emit_text(&mut self, texto: &str) {
        let st = *self.state();
        if st.skip || st.hidden {
            return;
        }
        if st.in_listtext {
            self.listtext.push_str(texto);
            return;
        }
        push_run(
            &mut self.runs,
            texto,
            st.bold.then_some(true),
            st.italic.then_some(true),
            st.underline.then_some(true),
        );
    }

    fn emit_char(&mut self, c: char) {
        let mut buf = [0; 4];
        self.emit_text(c.encode_utf8(&mut buf));
    }

    fn unicode(&mut self, n: i32) {
        let unidad = n.rem_euclid(0x10000) as u32;
        let previo = self.high.take();
        match (previo, unidad) {
            (_, 0xD800..=0xDBFF) => {
                if previo.is_some() {
                    self.emit_char(REEMPLAZO);
                }
                self.high = Some(unidad);
            }
            (Some(alto), 0xDC00..=0xDFFF) => {
                let punto = 0x10000 + ((alto - 0xD800) << 10) + (unidad - 0xDC00);
                self.emit_char(char::from_u32(punto).unwrap_or(REEMPLAZO));
            }
            _ => {
                if previo.is_some() {
                    self.emit_char(REEMPLAZO);
                }
                self.emit_char(char::from_u32(unidad).unwrap_or(REEMPLAZO));
            }
        }
        self.skip_units = self.state().uc;
    }

    fn par(&mut self) {
        if !self.state().intbl {
            self.para_end();
        } else if !self.runs.is_empty() {
            self.emit_text(" ");
        }
    }

    fn para_end(&mut self) {
        if self.split && self.runs.is_empty() {
            self.marker = None;
        } else {
            self.emit();
        }
        self.split = false;
        self.item = false;
    }

    fn line(&mut self) {
        if self.state().intbl {
            self.emit_text(" ");
        } else {
            self.emit();
            self.split = true;
        }
    }

    fn take_text(&mut self) -> String {
        self.marker = None;
        self.item = false;
        std::mem::take(&mut self.runs)
            .iter()
            .map(|run| run.text.as_str())
            .collect()
    }

    fn close_cell(&mut self) {
        let texto = self.take_text();
        self.cells.push(texto.trim_end().to_owned());
    }

    fn row_end(&mut self) {
        if !self.runs.is_empty() {
            self.close_cell();
        }
        let celdas = std::mem::take(&mut self.cells);
        self.out.push(Block::Paragraph {
            runs: vec![Run::plain(celdas.join("\t"))],
        });
    }

    fn emit(&mut self) {
        let st = *self.state();
        let mut runs = std::mem::take(&mut self.runs);
        let marcador = self.marker.take();
        let lista = self.item;
        if let Some(marcador) = marcador {
            runs.insert(0, Run::plain(marcador));
        }
        if runs.is_empty() {
            runs.push(Run::plain(String::new()));
        }
        let bloque = if lista {
            Block::ListItem { depth: st.ilvl, runs }
        } else if let Some(nivel) = st.outline {
            Block::Heading { level: nivel + 1, runs }
        } else {
            Block::Paragraph { runs }
        };
        self.out.push(bloque);
    }

    fn finish_marker(&mut self) {
        let texto = clean(&std::mem::take(&mut self.listtext));
        let texto = texto.trim_matches(|c: char| c == '\t' || c == ' ');
        self.item = true;
        self.marker = Some(if texto.chars().any(char::is_alphanumeric) {
            format!("{texto} ")
        } else {
            "• ".to_owned()
        });
    }

    fn finish(mut self) -> Vec<Block> {
        self.flush();
        if self.high.take().is_some() {
            self.emit_char(REEMPLAZO);
        }
        if !self.cells.is_empty() {
            self.row_end();
        } else if !self.runs.is_empty() {
            self.emit();
        }
        self.out
    }
}

fn hex_byte(bytes: &[u8], j: usize) -> Option<u8> {
    let alto = char::from(*bytes.get(j)?).to_digit(16)?;
    let bajo = char::from(*bytes.get(j + 1)?).to_digit(16)?;
    u8::try_from(alto * 16 + bajo).ok()
}

fn number(bytes: &[u8], k: usize) -> (Option<i32>, usize) {
    let ini = k + usize::from(bytes.get(k) == Some(&b'-'));
    let largo = bytes
        .get(ini..)
        .unwrap_or_default()
        .iter()
        .take_while(|b| b.is_ascii_digit())
        .count();
    if largo == 0 {
        return (None, k);
    }
    let fin = ini + largo;
    let valor = std::str::from_utf8(&bytes[k..fin])
        .ok()
        .and_then(|texto| texto.parse().ok());
    (valor, fin)
}

fn simbolo(nombre: &str) -> Option<&'static str> {
    Some(match nombre {
        "emdash" => "—",
        "endash" => "–",
        "bullet" => "•",
        "lquote" => "‘",
        "rquote" => "’",
        "ldblquote" => "“",
        "rdblquote" => "”",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::parse;
    use crate::model::Block;

    fn etiqueta(block: &Block) -> String {
        match block {
            Block::Paragraph { .. } => "p".to_owned(),
            Block::Heading { level, .. } => format!("h{level}"),
            Block::ListItem { depth, .. } => format!("li{depth}"),
            Block::Quote { .. } => "cita".to_owned(),
            Block::Image { .. } => "img".to_owned(),
        }
    }

    fn afirma(rtf: &str, esperado: &[(&str, &str)]) {
        let blocks = parse(rtf.as_bytes());
        let reales: Vec<(String, String)> = blocks
            .iter()
            .map(|block| {
                let texto: String = block.runs().iter().map(|run| run.text.as_str()).collect();
                (etiqueta(block), texto)
            })
            .collect();
        for block in &blocks {
            for run in block.runs() {
                assert!(!run.text.contains('\n'), "{:?}", run.text);
            }
        }
        let real: Vec<(&str, &str)> = reales
            .iter()
            .map(|(tipo, texto)| (tipo.as_str(), texto.as_str()))
            .collect();
        assert_eq!(real, esperado);
    }

    fn tramos(rtf: &str) -> Vec<(String, String)> {
        let blocks = parse(rtf.as_bytes());
        blocks
            .iter()
            .flat_map(Block::runs)
            .map(|run| {
                let mut estilo = String::new();
                if run.bold == Some(true) {
                    estilo.push('b');
                }
                if run.italic == Some(true) {
                    estilo.push('i');
                }
                if run.underline == Some(true) {
                    estilo.push('u');
                }
                (run.text.clone(), estilo)
            })
            .collect()
    }

    fn afirma_tramos(rtf: &str, esperado: &[(&str, &str)]) {
        let reales = tramos(rtf);
        let real: Vec<(&str, &str)> = reales
            .iter()
            .map(|(texto, estilo)| (texto.as_str(), estilo.as_str()))
            .collect();
        assert_eq!(real, esperado);
    }

    #[test]
    fn negrita_cursiva_y_subrayado() {
        afirma_tramos(
            r"{\rtf1 a\b b\b0 c\i d\i0 e\ul f\ul0 g\ulnone h}",
            &[("a", ""), ("b", "b"), ("c", ""), ("d", "i"), ("e", ""), ("f", "u"), ("gh", "")],
        );
    }

    #[test]
    fn plain_reinicia_el_caracter() {
        afirma_tramos(r"{\rtf1\b\i\ul x\plain y}", &[("x", "biu"), ("y", "")]);
    }

    #[test]
    fn par_y_line_parten_bloques_sin_saltos() {
        afirma(
            r#"{\rtf1 uno\par
dos\line tres\par}"#,
            &[("p", "uno"), ("p", "dos"), ("p", "tres")],
        );
    }

    #[test]
    fn par_vacio_en_medio_emite_bloque_vacio() {
        afirma(r"{\rtf1 a\par\par b}", &[("p", "a"), ("p", ""), ("p", "b")]);
    }

    #[test]
    fn par_final_no_deja_bloque_vacio() {
        afirma(r"{\rtf1 a\par}", &[("p", "a")]);
    }

    #[test]
    fn cp1252_por_defecto() {
        afirma(r"{\rtf1 caf\'e9}", &[("p", "café")]);
    }

    #[test]
    fn ansicpg1251_cambia_la_pagina() {
        afirma(r"{\rtf1\ansi\ansicpg1251 \'cf\'f0\'e8\'e2\'e5\'f2}", &[("p", "Привет")]);
    }

    #[test]
    fn ansicpg932_decodifica_pares_de_bytes() {
        afirma(r"{\rtf1\ansicpg932 \'82\'a0}", &[("p", "あ")]);
    }

    #[test]
    fn unicode_con_su_respaldo() {
        afirma(r"{\rtf1 a\u233 ?b}", &[("p", "aéb")]);
    }

    #[test]
    fn par_sustituto_forma_un_solo_caracter() {
        afirma(r"{\rtf1\uc1 \u-10179?\u-8704?}", &[("p", "\u{1F600}")]);
    }

    #[test]
    fn uc2_salta_dos_respaldos_de_bytes_o_caracteres() {
        afirma(r"{\rtf1\uc2 \u233\'3f\'3f x}", &[("p", "é x")]);
        afirma(concat!(r"{\rtf1\uc2 \u233", "ab c}"), &[("p", "é c")]);
    }

    #[test]
    fn uc0_no_salta_nada() {
        afirma(r"{\rtf1\uc0 \u233?}", &[("p", "é?")]);
    }

    #[test]
    fn llave_cancela_el_salto() {
        afirma(r"{\rtf1{\u233}x}", &[("p", "éx")]);
    }

    #[test]
    fn line_dentro_de_item_no_repite_marcador() {
        afirma(
            r"{\rtf1\pard{\listtext\'b7\tab}uno\line dos\par fin\par}",
            &[("li0", "• uno"), ("li0", "dos"), ("p", "fin")],
        );
    }

    #[test]
    fn palabra_de_control_cancela_el_salto() {
        afirma_tramos(r"{\rtf1\uc1 \u233\b ?x}", &[("é", ""), ("?x", "b")]);
    }

    #[test]
    fn destinos_omitidos() {
        afirma(
            r#"{\rtf1{\fonttbl{\f0\fnil Calibri;}}{\colortbl;\red0\green0\blue0;}{\stylesheet{\s0 Normal;}}{\info{\title Doc}}{\*\generator Riched20;}{\fldinst HYPERLINK "x"}Texto}"#,
            &[("p", "Texto")],
        );
    }

    #[test]
    fn pict_se_omite_con_sus_bytes_binarios() {
        afirma(r"{\rtf1 A{\pict\wmetafile8\bin4 }}}}}B}", &[("p", "AB")]);
    }

    #[test]
    fn outlinelevel_da_titulos() {
        afirma(
            r"{\rtf1\pard\outlinelevel0 T\'edtulo\par\pard\outlinelevel2 Sub\par}",
            &[("h1", "Título"), ("h3", "Sub")],
        );
    }

    #[test]
    fn listtext_de_vineta_es_bullet() {
        afirma(
            r"{\rtf1\pard{\listtext\pard\plain\'b7\tab}Elemento\par}",
            &[("li0", "• Elemento")],
        );
    }

    #[test]
    fn listtext_numerado_toma_el_nivel_exterior() {
        afirma(r"{\rtf1\pard\ilvl1{\listtext\pard 1.\tab}Uno\par}", &[("li1", "1. Uno")]);
    }

    #[test]
    fn pntext_de_wordpad_es_bullet() {
        afirma(
            r"{\rtf1\pard{\pntext\'b7\tab}{\*\pn\pnlvlblt\pnf1}\fi-360\li720 Punto\par}",
            &[("li0", "• Punto")],
        );
    }

    #[test]
    fn tabla_une_celdas_y_par_dentro_de_celda_es_espacio() {
        afirma(r"{\rtf1\trowd\intbl a\par b\cell c\cell\row}", &[("p", "a b\tc")]);
    }

    #[test]
    fn simbolos_de_escape_y_guiones() {
        afirma(r"{\rtf1 \{a\} \\ b\~c\-d\_e\emdash f}", &[("p", "{a} \\ b\u{a0}cd-e—f")]);
    }

    #[test]
    fn campo_muestra_su_resultado() {
        afirma(r#"{\rtf1 {\field{\*\fldinst HYPERLINK "x"}{\fldrslt texto}}}"#, &[("p", "texto")]);
        afirma(r#"{\rtf1 {\field{\fldinst HYPERLINK "x"}{\fldrslt texto}}}"#, &[("p", "texto")]);
    }

    #[test]
    fn texto_oculto_no_se_emite() {
        afirma(r"{\rtf1 visible{\v oculto} fin}", &[("p", "visible fin")]);
    }

    #[test]
    fn rtf_truncado_conserva_el_texto() {
        afirma_tramos(r"{\rtf1 hola \b mundo", &[("hola ", ""), ("mundo", "b")]);
    }

    #[test]
    fn llave_sobrante_no_rompe_el_texto() {
        afirma(r"{\rtf1 a}}b}", &[("p", "ab")]);
    }

    #[test]
    fn wordpad_real_con_negrita_y_tildes() {
        let rtf = r"{\rtf1\ansi\ansicpg1252\deff0\nouicompat\deflang3082{\fonttbl{\f0\fnil\fcharset0 Calibri;}}{\*\generator Riched20 10.0.22621}\viewkind4\uc1 \pard\sa200\sl276\slmult1\f0\fs22\lang10 Hola \b mundo\b0  y caf\'e9.\par}";
        afirma(rtf, &[("p", "Hola mundo y café.")]);
        afirma_tramos(rtf, &[("Hola ", ""), ("mundo", "b"), (" y café.", "")]);
    }
}
