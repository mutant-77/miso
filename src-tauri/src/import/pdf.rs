use std::fs;
use std::ops::Range;
use std::path::Path;
use std::sync::OnceLock;

use pdfium_render::prelude::*;

use super::{ParseError, clean};
use crate::model::{Block, Run};

const DLL: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/pdfium/pdfium.dll"));

static PDFIUM: OnceLock<Result<Pdfium, String>> = OnceLock::new();

fn pdfium() -> Result<&'static Pdfium, String> {
    PDFIUM
        .get_or_init(|| cargar().inspect_err(|err| log::error!("PDFium: {err}")))
        .as_ref()
        .map_err(Clone::clone)
}

fn cargar() -> Result<Pdfium, String> {
    let ruta = std::env::temp_dir().join("miso-pdfium-8125.dll");
    let vigente = fs::metadata(&ruta).is_ok_and(|m| m.len() == DLL.len() as u64);
    if !vigente {
        extraer(&ruta)?;
    }
    let bindings = Pdfium::bind_to_library(&ruta).map_err(|err| format!("{}: {err}", ruta.display()))?;
    Ok(Pdfium::new(bindings))
}

// Escribe en un `.tmp` y renombra, para no dejar una DLL a medias.
fn extraer(ruta: &Path) -> Result<(), String> {
    let tmp = ruta.with_extension("dll.tmp");
    fs::write(&tmp, DLL).map_err(|err| format!("{}: {err}", tmp.display()))?;
    fs::rename(&tmp, ruta).map_err(|err| format!("{}: {err}", ruta.display()))
}

pub fn parse(bytes: &[u8]) -> Result<Vec<Block>, ParseError> {
    let pdfium = pdfium().map_err(ParseError::Internal)?;
    let documento = pdfium
        .load_pdf_from_byte_slice(bytes, None)
        .map_err(|err| match err {
            PdfiumError::PdfiumLibraryInternalError(PdfiumInternalError::PasswordError) => {
                ParseError::Protected
            }
            otro => ParseError::Corrupt(otro.to_string()),
        })?;
    let mut textos: Vec<String> = Vec::new();
    let mut cola: Option<Linea> = None;
    for pagina in documento.pages().iter() {
        let glifos = caracteres(&pagina).map_err(|err| ParseError::Corrupt(err.to_string()))?;
        let mut lineas = lineas(&glifos);
        let mut nuevos = parrafos(&lineas).into_iter();
        if let (Some(previa), Some(primera), Some(ultimo)) = (&cola, lineas.first(), textos.last_mut())
            && sigue(previa, primera)
            && let Some(primero) = nuevos.next()
        {
            // En el último renglón de la página PDFium no marca el guion con `is_hyphen`.
            unir(ultimo, previa.guion || previa.texto.ends_with('-'), &primero);
        }
        textos.extend(nuevos);
        if let Some(ultima) = lineas.pop() {
            cola = Some(ultima);
        }
    }
    if textos.is_empty() {
        return Err(ParseError::Unsupported);
    }
    Ok(textos
        .iter()
        .map(|texto| Block::Paragraph {
            runs: vec![Run::plain(clean(texto))],
        })
        .collect())
}

// Continuación evidente entre páginas: acaba en guion, o sin fin de frase y la siguiente empieza en minúscula.
fn sigue(previa: &Linea, siguiente: &Linea) -> bool {
    previa.texto.ends_with('-')
        || (!previa.texto.ends_with(['.', '?', '!', '…', '»', '"', '”'])
            && siguiente.texto.chars().next().is_some_and(char::is_lowercase))
}

#[derive(Clone, Copy, Default)]
struct Caja {
    left: f32,
    right: f32,
    top: f32,
    bottom: f32,
}

impl Caja {
    fn une(self, otra: Caja) -> Caja {
        Caja {
            left: self.left.min(otra.left),
            right: self.right.max(otra.right),
            top: self.top.max(otra.top),
            bottom: self.bottom.min(otra.bottom),
        }
    }
}

struct Glifo {
    c: char,
    corte: bool,
    guion: bool,
    caja: Option<Caja>,
}

fn caracteres(pagina: &PdfPage) -> Result<Vec<Glifo>, PdfiumError> {
    let texto = pagina.text()?;
    let chars = texto.chars();
    Ok(chars
        .iter()
        .filter_map(|ch| {
            let c = ch.unicode_char()?;
            Some(Glifo {
                c,
                corte: matches!(c, '\r' | '\n') && matches!(ch.is_generated(), Ok(true)),
                guion: matches!(ch.is_hyphen(), Ok(true)),
                caja: ch.loose_bounds().ok().map(|r| Caja {
                    left: r.left().value,
                    right: r.right().value,
                    top: r.top().value,
                    bottom: r.bottom().value,
                }),
            })
        })
        .collect())
}

struct Linea {
    texto: String,
    guion: bool,
    left: f32,
    right: f32,
    top: f32,
    bottom: f32,
    // Ancho de la primera palabra y del espacio que la sigue.
    palabra: f32,
}

fn union(a: Option<Caja>, b: Option<Caja>) -> Option<Caja> {
    match (a, b) {
        (Some(a), Some(b)) => Some(a.une(b)),
        (a, b) => a.or(b),
    }
}

#[derive(Default)]
struct Acumulado {
    texto: String,
    guion: bool,
    caja: Option<Caja>,
    palabra: Option<Caja>,
    hecha: bool,
}

impl Acumulado {
    fn sumar(&mut self, c: char, caja: Option<Caja>) {
        self.texto.push(c);
        self.caja = union(self.caja, caja);
        if !self.hecha && (self.palabra.is_some() || !c.is_whitespace()) {
            self.palabra = union(self.palabra, caja);
            self.hecha = c.is_whitespace();
        }
    }

    fn cerrar(&mut self, lineas: &mut Vec<Linea>) {
        let texto = self.texto.trim();
        if !texto.is_empty() {
            let c = self.caja.unwrap_or_default();
            lineas.push(Linea {
                texto: texto.to_owned(),
                guion: self.guion,
                left: c.left,
                right: c.right,
                top: c.top,
                bottom: c.bottom,
                palabra: self.palabra.map_or(0.0, |p| p.right - p.left),
            });
        }
        *self = Self::default();
    }
}

fn descartable(c: char) -> bool {
    matches!(c, '\u{FFFE}' | '\u{FFFF}') || (c.is_control() && c != '\t')
}

// PDFium funde el guion de fin de línea y su salto en un U+0002 con `is_hyphen`.
fn lineas(glifos: &[Glifo]) -> Vec<Linea> {
    let mut out = Vec::new();
    let mut acc = Acumulado::default();
    for g in glifos {
        if g.corte {
            acc.cerrar(&mut out);
        } else if g.guion {
            acc.sumar('-', g.caja);
            acc.guion = true;
            acc.cerrar(&mut out);
        } else if !descartable(g.c) {
            acc.sumar(g.c, g.caja);
        }
    }
    acc.cerrar(&mut out);
    out
}

fn subida(act: &Linea, sig: &Linea) -> bool {
    sig.bottom > act.top
}

fn hueco(act: &Linea, sig: &Linea) -> f32 {
    act.bottom - sig.top
}

fn tramos(lineas: &[Linea]) -> Vec<Range<usize>> {
    let mut out = Vec::new();
    let mut inicio = 0;
    for i in 1..lineas.len() {
        if subida(&lineas[i - 1], &lineas[i]) {
            out.push(inicio..i);
            inicio = i;
        }
    }
    if inicio < lineas.len() {
        out.push(inicio..lineas.len());
    }
    out
}

fn mediana(v: &mut [f32]) -> f32 {
    if v.is_empty() {
        return 0.0;
    }
    v.sort_by(f32::total_cmp);
    let n = v.len();
    if n % 2 == 1 {
        v[n / 2]
    } else {
        (v[n / 2 - 1] + v[n / 2]) / 2.0
    }
}

fn percentil(v: &mut [f32], p: f32) -> f32 {
    if v.is_empty() {
        return 0.0;
    }
    v.sort_by(f32::total_cmp);
    let rango = ((p * v.len() as f32).ceil() as usize).clamp(1, v.len());
    v[rango - 1]
}

struct Estilo {
    h: f32,
    r: f32,
    l: f32,
    g: f32,
}

impl Estilo {
    fn de(tramo: &[Linea]) -> Estilo {
        let mut alturas: Vec<f32> = tramo.iter().map(|l| l.top - l.bottom).collect();
        let h = mediana(&mut alturas);
        if tramo.len() < 3 {
            let r = tramo.iter().map(|ln| ln.right).fold(f32::MIN, f32::max);
            let l = tramo.iter().map(|ln| ln.left).fold(f32::MAX, f32::min);
            return Estilo { h, r, l, g: h };
        }
        let mut huecos: Vec<f32> = tramo.windows(2).map(|p| hueco(&p[0], &p[1])).collect();
        let mut derechas: Vec<f32> = tramo.iter().map(|l| l.right).collect();
        let mut izquierdas: Vec<f32> = tramo.iter().map(|l| l.left).collect();
        Estilo {
            h,
            r: percentil(&mut derechas, 0.9),
            l: percentil(&mut izquierdas, 0.1),
            g: mediana(&mut huecos),
        }
    }
}

fn cortes(lineas: &[Linea]) -> Vec<bool> {
    let mut cortes = vec![false; lineas.len()];
    for tramo in tramos(lineas) {
        let estilo = Estilo::de(&lineas[tramo.clone()]);
        for i in tramo.clone() {
            let act = &lineas[i];
            // En texto sin justificar, una línea corta sigue si la palabra siguiente no cabía en ella.
            let corta = act.right < estilo.r - 2.0 * estilo.h
                && lineas.get(i + 1).is_none_or(|sig| estilo.r - act.right > sig.palabra);
            cortes[i] = if i + 1 == lineas.len() {
                true
            } else if i + 1 == tramo.end {
                corta
            } else {
                let sig = &lineas[i + 1];
                corta
                    || hueco(act, sig) > estilo.g + 0.5 * estilo.h
                    || sig.left > estilo.l + estilo.h
            };
        }
    }
    cortes
}

fn unir(actual: &mut String, guion: bool, inicio: &str) {
    if guion && inicio.chars().next().is_some_and(char::is_lowercase) {
        actual.pop();
    } else {
        actual.push(' ');
    }
    actual.push_str(inicio);
}

fn parrafos(lineas: &[Linea]) -> Vec<String> {
    let cortes = cortes(lineas);
    let mut out = Vec::new();
    let mut actual = String::new();
    for (i, linea) in lineas.iter().enumerate() {
        if actual.is_empty() {
            actual.push_str(&linea.texto);
        } else {
            unir(&mut actual, lineas[i - 1].guion, &linea.texto);
        }
        if cortes[i] {
            out.push(std::mem::take(&mut actual));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::AppError;

    const LINEA: f32 = 14.0;
    const MARGEN: f32 = 72.0;

    type Renglon = (f32, f32, String);

    fn ancho(s: &str, n: usize) -> String {
        let palabras: Vec<&str> = s.split_whitespace().collect();
        let letras: usize = palabras.iter().map(|p| p.chars().count()).sum();
        let huecos = palabras.len() - 1;
        assert!(letras + huecos <= n, "{s}");
        let espacios = n - letras;
        let mut out = String::new();
        for (i, p) in palabras.iter().enumerate() {
            out.push_str(p);
            if i < huecos {
                let extra = usize::from(i < espacios % huecos);
                out.push_str(&" ".repeat(espacios / huecos + extra));
            }
        }
        assert_eq!(out.chars().count(), n, "{s}");
        out
    }

    fn bloque(x: f32, y: f32, textos: Vec<String>) -> Vec<Renglon> {
        textos
            .into_iter()
            .enumerate()
            .map(|(i, t)| (x, y - LINEA * i as f32, t))
            .collect()
    }

    fn pdf(paginas: &[Vec<Renglon>]) -> Vec<u8> {
        let pdfium = pdfium().expect("PDFium");
        let mut documento = pdfium.create_new_pdf().unwrap();
        let courier = documento.fonts_mut().courier();
        for renglones in paginas {
            let mut pagina = documento
                .pages_mut()
                .create_page_at_end(PdfPagePaperSize::new_portrait(
                    PdfPagePaperStandardSize::USLetterAnsiA,
                ))
                .unwrap();
            for (x, y, texto) in renglones {
                pagina
                    .objects_mut()
                    .create_text_object(
                        PdfPoints::new(*x),
                        PdfPoints::new(*y),
                        texto,
                        courier,
                        PdfPoints::new(12.0),
                    )
                    .unwrap();
            }
        }
        documento.save_to_bytes().unwrap()
    }

    fn parrafos_de(bytes: &[u8]) -> Vec<String> {
        let Ok(blocks) = parse(bytes) else {
            panic!("el PDF debía importarse");
        };
        blocks
            .iter()
            .map(|block| {
                let runs = block.runs();
                assert_eq!(runs.len(), 1);
                let texto = &runs[0].text;
                assert!(!texto.contains('\n') && !texto.contains('\u{FFFE}'), "{texto:?}");
                texto.split_whitespace().collect::<Vec<_>>().join(" ")
            })
            .collect()
    }

    #[test]
    fn dos_parrafos_separados_por_hueco() {
        let primero = bloque(
            MARGEN,
            700.0,
            vec![
                ancho("alfa beta gamma delta epsilon", 60),
                ancho("zeta eta theta iota kappa", 60),
                ancho("lambda mu nu xi omicron pi", 60),
            ],
        );
        let segundo = bloque(
            MARGEN,
            700.0 - 4.0 * LINEA,
            vec![
                ancho("rho sigma tau upsilon phi chi", 60),
                "psi omega fin del segundo parrafo".to_string(),
            ],
        );
        let bytes = pdf(&[[primero, segundo].concat()]);
        assert_eq!(
            parrafos_de(&bytes),
            [
                "alfa beta gamma delta epsilon zeta eta theta iota kappa lambda mu nu xi omicron pi",
                "rho sigma tau upsilon phi chi psi omega fin del segundo parrafo",
            ]
        );
    }

    #[test]
    fn ultima_linea_corta_corta_el_parrafo() {
        let primero = bloque(
            MARGEN,
            700.0,
            vec![
                ancho("uno dos tres cuatro cinco seis", 60),
                ancho("siete ocho nueve diez once doce", 60),
                ancho("trece catorce quince dieciseis", 60),
                "fin corto.".to_string(),
            ],
        );
        let segundo = bloque(
            MARGEN,
            700.0 - 4.0 * LINEA,
            vec![
                ancho("nuevo parrafo sin sangria, linea uno", 60),
                ancho("nuevo parrafo sin sangria, linea dos", 60),
            ],
        );
        let bytes = pdf(&[[primero, segundo].concat()]);
        assert_eq!(
            parrafos_de(&bytes),
            [
                "uno dos tres cuatro cinco seis siete ocho nueve diez once doce trece catorce quince dieciseis fin corto.",
                "nuevo parrafo sin sangria, linea uno nuevo parrafo sin sangria, linea dos",
            ]
        );
    }

    #[test]
    fn linea_corta_sin_justificar_sigue_si_la_palabra_no_cabia() {
        let bytes = pdf(&[bloque(
            MARGEN,
            700.0,
            vec![
                ancho("ser negligente, ser atraido, es una manera de", 60),
                ancho("si estuviera presente, la ley seria la suave", 52),
                ancho("interioridad de la conciencia y la ley sigue", 58),
                ancho("su curso en el texto hasta el final de la", 59),
                "frase.".to_string(),
            ],
        )]);
        assert_eq!(
            parrafos_de(&bytes),
            ["ser negligente, ser atraido, es una manera de si estuviera presente, la ley seria la suave interioridad de la conciencia y la ley sigue su curso en el texto hasta el final de la frase."]
        );
    }

    #[test]
    fn sangria_de_primera_linea_corta_el_parrafo() {
        let primero = bloque(
            MARGEN,
            700.0,
            vec![
                ancho("uno dos tres cuatro cinco seis", 60),
                ancho("siete ocho nueve diez once doce", 60),
                ancho("trece catorce quince dieciseis", 60),
            ],
        );
        let segundo = [
            bloque(108.0, 700.0 - 3.0 * LINEA, vec![ancho("sangria de primera linea del nuevo parrafo", 56)]),
            bloque(MARGEN, 700.0 - 4.0 * LINEA, vec![ancho("continua el parrafo con margen normal", 60)]),
        ]
        .concat();
        let bytes = pdf(&[[primero, segundo].concat()]);
        assert_eq!(
            parrafos_de(&bytes),
            [
                "uno dos tres cuatro cinco seis siete ocho nueve diez once doce trece catorce quince dieciseis",
                "sangria de primera linea del nuevo parrafo continua el parrafo con margen normal",
            ]
        );
    }

    #[test]
    fn guion_de_final_de_linea_une_la_palabra() {
        let bytes = pdf(&[bloque(
            MARGEN,
            700.0,
            vec![
                format!("{:<55}pala-", "texto de prueba para comprobar el guion"),
                "bra que se parte al final de la linea.".to_string(),
            ],
        )]);
        assert_eq!(
            parrafos_de(&bytes),
            ["texto de prueba para comprobar el guion palabra que se parte al final de la linea."]
        );
    }

    #[test]
    fn guion_seguido_de_mayuscula_se_conserva() {
        let bytes = pdf(&[bloque(
            MARGEN,
            700.0,
            vec![
                format!("{:<54}Norte-", "informe del consejo sobre la region del"),
                "Sur que se conserva tal cual en el texto.".to_string(),
            ],
        )]);
        assert_eq!(
            parrafos_de(&bytes),
            ["informe del consejo sobre la region del Norte- Sur que se conserva tal cual en el texto."]
        );
    }

    fn columna(x: f32, nombre: &str, final_corto: bool) -> Vec<Renglon> {
        let mut textos: Vec<String> =
            (1..=8).map(|i| ancho(&format!("columna {nombre} {i} texto"), 30)).collect();
        if final_corto {
            textos.push("fin.".to_string());
        }
        bloque(x, 700.0, textos)
    }

    fn esperado_columna(nombre: &str, final_corto: bool) -> String {
        let mut partes: Vec<String> = (1..=8).map(|i| format!("columna {nombre} {i} texto")).collect();
        if final_corto {
            partes.push("fin.".to_string());
        }
        partes.join(" ")
    }

    #[test]
    fn dos_columnas_de_un_parrafo_cada_una() {
        let bytes = pdf(&[[columna(MARGEN, "izq", true), columna(324.0, "der", true)].concat()]);
        assert_eq!(
            parrafos_de(&bytes),
            [esperado_columna("izq", true), esperado_columna("der", true)]
        );
    }

    #[test]
    fn parrafo_que_sigue_en_la_otra_columna_no_se_corta() {
        let bytes = pdf(&[[columna(MARGEN, "izq", false), columna(324.0, "der", false)].concat()]);
        assert_eq!(
            parrafos_de(&bytes),
            [format!("{} {}", esperado_columna("izq", false), esperado_columna("der", false))]
        );
    }

    #[test]
    fn parrafo_que_cruza_pagina_se_une() {
        let uno = bloque(
            MARGEN,
            700.0,
            vec![
                ancho("mas aun \u{2014}desde Igitur\u{2014} como el", 60),
                ancho("movimiento depositado en nuestro", 60),
            ],
        );
        let dos = bloque(MARGEN, 700.0, vec![ancho("pensamiento, para el siglo", 60), "venidero.".to_string()]);
        let bytes = pdf(&[uno, dos]);
        assert_eq!(
            parrafos_de(&bytes),
            ["mas aun \u{2014}desde Igitur\u{2014} como el movimiento depositado en nuestro pensamiento, para el siglo venidero."]
        );
    }

    #[test]
    fn corte_legitimo_entre_paginas_se_conserva() {
        let uno = bloque(MARGEN, 700.0, vec![ancho("pagina uno, linea uno", 60), ancho("acaba la frase.", 60)]);
        let dos = bloque(MARGEN, 700.0, vec![ancho("sigue en minuscula tras punto", 60), ancho("pero sin punto final", 60)]);
        let tres = bloque(MARGEN, 700.0, vec![ancho("Capitulo dos de la obra", 60), "fin.".to_string()]);
        let bytes = pdf(&[uno, dos, tres]);
        assert_eq!(
            parrafos_de(&bytes),
            [
                "pagina uno, linea uno acaba la frase.",
                "sigue en minuscula tras punto pero sin punto final",
                "Capitulo dos de la obra fin.",
            ]
        );
    }

    #[test]
    fn dos_puntos_o_guion_al_final_de_pagina_siguen() {
        let uno = bloque(MARGEN, 700.0, vec![ancho("la espera se mantiene", 60), ancho("como una espera:", 60)]);
        let dos = bloque(MARGEN, 700.0, vec![ancho("atencion aguda a lo nuevo", 60), ancho("en la ciudad de Le tres-", 60)]);
        let tres = bloque(MARGEN, 700.0, vec![ancho("Haut, ni en el sanatorio", 60), "fin.".to_string()]);
        let bytes = pdf(&[uno, dos, tres]);
        assert_eq!(
            parrafos_de(&bytes),
            ["la espera se mantiene como una espera: atencion aguda a lo nuevo en la ciudad de Le tres- Haut, ni en el sanatorio fin."]
        );
    }

    #[test]
    fn guion_entre_paginas_une_la_palabra() {
        let uno = bloque(
            MARGEN,
            700.0,
            vec![
                ancho("texto de prueba de la pagina uno", 60),
                format!("{:<55}pala-", "palabra partida al final de la"),
            ],
        );
        let dos = bloque(MARGEN, 700.0, vec![ancho("bra que sigue en la pagina dos", 60), "fin.".to_string()]);
        let bytes = pdf(&[uno, dos]);
        assert_eq!(
            parrafos_de(&bytes),
            ["texto de prueba de la pagina uno palabra partida al final de la palabra que sigue en la pagina dos fin."]
        );
    }

    #[test]
    fn dll_extraida_a_temp_con_el_tamano_embebido() {
        let bytes = pdf(&[bloque(MARGEN, 700.0, vec!["texto para extraer la DLL.".to_string()])]);
        parrafos_de(&bytes);
        let ruta = std::env::temp_dir().join("miso-pdfium-8125.dll");
        assert_eq!(std::fs::metadata(&ruta).unwrap().len(), DLL.len() as u64);
    }

    #[test]
    fn pdf_sin_texto_es_unsupported() {
        assert!(matches!(parse(&pdf(&[Vec::new()])), Err(ParseError::Unsupported)));
    }

    #[test]
    fn pdf_basura_es_error_sin_panico() {
        assert!(matches!(
            parse(b"%PDF-1.7 basura"),
            Err(ParseError::Corrupt(_) | ParseError::Unsupported)
        ));
    }

    #[test]
    fn pdf_protegido_es_protected() {
        let bytes = include_bytes!("../../../tests/fixtures/protegido.pdf");
        assert!(matches!(parse(bytes), Err(ParseError::Protected)));
        let err = super::super::from_bytes("protegido.pdf", bytes).unwrap_err();
        assert!(matches!(err, AppError::Protected { ref file } if file == "protegido.pdf"));
    }
}

