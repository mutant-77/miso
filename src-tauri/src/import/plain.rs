use std::borrow::Cow;

use chardetng::{EncodingDetector, Iso2022JpDetection, Utf8Detection};
use encoding_rs::Encoding;

use crate::model::{Block, Run};

const SNIFF_LEN: usize = 8 * 1024;
// Por debajo no hay muestra para decidir que el texto viene cortado a un ancho fijo.
const MIN_PAIRS: usize = 8;
// La prosa se corta a 60–80 caracteres; líneas regulares más cortas son versos o listas.
const MIN_WIDTH: usize = 50;

#[derive(Debug)]
pub struct Unsupported;

pub fn parse(bytes: &[u8]) -> Result<Vec<Block>, Unsupported> {
    Ok(blocks(&decode(bytes)?))
}

pub fn blocks(text: &str) -> Vec<Block> {
    if text.is_empty() {
        return Vec::new();
    }
    let text = text
        .replace("\r\n", "\n")
        .replace('\r', "\n")
        // El guion blando es exclusivo de la partición del frontend (anchor.ts no lo cuenta).
        .replace(['\u{FFFC}', '\u{AD}'], "");
    let lines: Vec<&str> = text.split('\n').collect();
    let soft = soft_breaks(&lines);
    let mut out = Vec::new();
    let mut paragraph = String::new();
    for (line, soft) in lines.iter().zip(soft) {
        paragraph.push_str(line);
        // Un espacio por cada salto: los índices de carácter (posiciones guardadas) no cambian.
        if soft {
            paragraph.push(' ');
        } else {
            out.push(Block::Paragraph {
                runs: vec![Run::plain(std::mem::take(&mut paragraph))],
            });
        }
    }
    out
}

/// Saltos puestos por un corte a ancho fijo: la primera palabra de la línea siguiente no cabía.
/// Sólo se aplican si son la mayoría; si no, el texto no viene cortado y cada línea es un párrafo.
fn soft_breaks(lines: &[&str]) -> Vec<bool> {
    let len = |s: &str| s.chars().count();
    let mut soft = vec![false; lines.len()];
    let mut widths: Vec<usize> = lines.iter().map(|l| len(l.trim_end())).filter(|&n| n > 0).collect();
    if widths.is_empty() {
        return soft;
    }
    widths.sort_unstable();
    let width = widths[(widths.len() - 1) * 95 / 100];
    if width < MIN_WIDTH {
        return soft;
    }
    let mut pairs = 0;
    for (i, pair) in lines.windows(2).enumerate() {
        let (line, next) = (pair[0].trim_end(), pair[1]);
        let Some(word) = next.split_whitespace().next() else { continue };
        if line.is_empty() {
            continue;
        }
        pairs += 1;
        soft[i] = !next.starts_with(char::is_whitespace) && len(line) + 1 + len(word) > width;
    }
    let joined = soft.iter().filter(|&&s| s).count();
    if pairs < MIN_PAIRS || joined * 2 < pairs {
        soft.fill(false);
    }
    soft
}

pub(crate) fn decode(bytes: &[u8]) -> Result<Cow<'_, str>, Unsupported> {
    if let Some((encoding, bom_len)) = Encoding::for_bom(bytes) {
        return Ok(encoding.decode_without_bom_handling(&bytes[bom_len..]).0);
    }
    if bytes[..bytes.len().min(SNIFF_LEN)].contains(&0) {
        return Err(Unsupported);
    }
    if let Ok(text) = std::str::from_utf8(bytes) {
        return Ok(Cow::Borrowed(text));
    }
    let mut detector = EncodingDetector::new(Iso2022JpDetection::Deny);
    detector.feed(bytes, true);
    let encoding = detector.guess(None, Utf8Detection::Allow);
    Ok(encoding.decode_without_bom_handling(bytes).0)
}

#[cfg(test)]
mod tests {
    use super::blocks;

    #[test]
    fn blocks_normaliza_saltos_y_quita_objetos() {
        let bloques = blocks("a\r\nb\rc\u{FFFC}\u{AD}");
        let textos: Vec<&str> = bloques
            .iter()
            .map(|block| block.runs()[0].text.as_str())
            .collect();
        assert_eq!(textos, ["a", "b", "c"]);
    }

    fn textos(text: &str) -> Vec<String> {
        blocks(text)
            .into_iter()
            .map(|block| block.runs()[0].text.clone())
            .collect()
    }

    const NOVELA: [&str; 14] = [
        "She stood at her bedroom window before going downstairs to take up the",
        "burden of a new day. She was just seventeen, but they did not keep any",
        "account of anniversaries at Hickory Farm. The sun had given her a loving",
        "glance as he lifted his bright old face above the horizon, but her",
        "father was too busy and careworn to remember, and, since her mother had",
        "gone away, there was no one else. She had read of the birthdays of other",
        "girls, full of strange, sweet surprises, and tender thoughts--but those",
        "were girls with mothers. A smile like a stray beam of sunshine drifted",
        "over her troubled young face, at the thought of the second Mrs Harding",
        "stopping for one instant in her round of ponderous toil to note the fact",
        "that one of her family had reached another milestone in life's journey.",
        "Certainly not on washing day, when every energy was absorbed in the",
        "elimination of impurity from her household linen, and life looked",
        "grotesque and hazy through clouds of soapy steam.",
    ];

    #[test]
    fn texto_cortado_a_ancho_fijo_se_une_en_parrafos() {
        let text = format!("_Chapter I_\n\n{}\n\nShe heard her father.", NOVELA.join("\n"));
        assert_eq!(
            textos(&text),
            ["_Chapter I_", "", &NOVELA.join(" "), "", "She heard her father."]
        );
    }

    #[test]
    fn unir_conserva_la_longitud_utf16() {
        let text = format!("{}\n\n{}", NOVELA.join("\r\n"), NOVELA.join("\n"));
        let unido = textos(&text).join("\n");
        assert_eq!(unido.encode_utf16().count(), text.replace("\r\n", "\n").encode_utf16().count());
    }

    #[test]
    fn poema_y_parrafos_por_linea_no_se_unen() {
        let poema = "Caminante, son tus huellas\nel camino y nada más;\nCaminante, no hay camino,\nse hace camino al andar.\nAl andar se hace el camino,\ny al volver la vista atrás\nse ve la senda que nunca\nse ha de volver a pisar.\nCaminante no hay camino\nsino estelas en la mar.";
        assert_eq!(textos(poema).len(), 10);
        let parrafos: Vec<String> = [9, 3, 14, 6, 1, 11, 4, 8, 2, 12]
            .iter()
            .map(|&n| "palabra ".repeat(n * 10).trim_end().to_owned())
            .collect();
        assert_eq!(textos(&parrafos.join("\n")), parrafos);
    }

    #[test]
    fn linea_sangrada_no_se_une() {
        let mut lineas = NOVELA.to_vec();
        lineas[11] = "    Certainly not on washing day, when every energy was absorbed in the";
        let t = textos(&lineas.join("\n"));
        assert_eq!(t.len(), 2);
        assert!(t[1].starts_with("    Certainly"));
    }
}
