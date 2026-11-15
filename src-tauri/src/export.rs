use crate::model::{Block, Doc, Run};

const CABECERA: &str = r"{\rtf1\ansi\ansicpg1252\deff0\uc1{\fonttbl{\f0\froman Times New Roman;}}";

pub fn plain(doc: &Doc) -> String {
    doc.blocks
        .iter()
        .filter(|block| !matches!(block, Block::Image { .. }))
        .map(|block| block.runs().iter().map(|run| run.text.as_str()).collect::<String>())
        .collect::<Vec<_>>()
        .join("\r\n")
}

pub fn rtf(doc: &Doc) -> String {
    let mut out = String::from(CABECERA);
    for block in &doc.blocks {
        if let Some(texto) = bloque(block) {
            out.push_str(&texto);
            out.push('\n');
        }
    }
    out.push('}');
    out
}

fn bloque(block: &Block) -> Option<String> {
    let (formato, runs) = match block {
        Block::Paragraph { runs } => (String::new(), runs),
        Block::Heading { level, runs } => {
            (format!(r"\outlinelevel{}\b", level.saturating_sub(1)), runs)
        }
        Block::ListItem { depth, runs } => (format!(r"\li{}", (u32::from(*depth) + 1) * 360), runs),
        Block::Quote { runs } => (r"\li720\ri720".to_owned(), runs),
        Block::Image { .. } => return None,
    };
    let mut out = format!(r"\pard\plain{formato} ");
    for run in runs {
        out.push_str(&tramo(run));
    }
    out.push_str(r"\par");
    Some(out)
}

fn tramo(run: &Run) -> String {
    let mut estilo = String::new();
    for (activo, control) in [(run.bold, r"\b"), (run.italic, r"\i"), (run.underline, r"\ul")] {
        if activo == Some(true) {
            estilo.push_str(control);
        }
    }
    let texto = escapa(&run.text);
    if estilo.is_empty() {
        texto
    } else {
        format!("{{{estilo} {texto}}}")
    }
}

fn escapa(texto: &str) -> String {
    let mut out = String::with_capacity(texto.len());
    for c in texto.chars() {
        match c {
            '\\' => out.push_str(r"\\"),
            '{' => out.push_str(r"\{"),
            '}' => out.push_str(r"\}"),
            '\t' => out.push_str(r"\tab "),
            '\u{a0}' => out.push_str(r"\~"),
            ' '..='~' => out.push(c),
            _ => {
                let mut unidades = [0; 2];
                for unidad in c.encode_utf16(&mut unidades) {
                    out.push_str(&format!(r"\u{}?", *unidad as i16));
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{CABECERA, plain, rtf};
    use crate::import::rtf::parse;
    use crate::model::{Block, Composition, Doc, Run};

    fn documento(blocks: Vec<Block>) -> Doc {
        Doc {
            title: "t.txt".into(),
            initial_char_index: 0,
            composition: Composition::Text,
            chapters: Vec::new(),
            blocks,
            images: Vec::new(),
        }
    }

    fn plano(text: &str) -> Run {
        Run::plain(text.into())
    }

    fn con(text: &str, bold: bool, italic: bool, underline: bool) -> Run {
        Run {
            text: text.into(),
            bold: bold.then_some(true),
            italic: italic.then_some(true),
            underline: underline.then_some(true),
        }
    }

    fn textos(blocks: &[Block]) -> Vec<String> {
        blocks
            .iter()
            .map(|block| block.runs().iter().map(|run| run.text.as_str()).collect())
            .collect()
    }

    fn estilos(blocks: &[Block]) -> Vec<(char, bool, bool, bool)> {
        blocks
            .iter()
            .flat_map(Block::runs)
            .flat_map(|run| {
                let (b, i, u) = (
                    run.bold == Some(true),
                    run.italic == Some(true),
                    run.underline == Some(true),
                );
                run.text.chars().map(move |c| (c, b, i, u))
            })
            .collect()
    }

    fn etiquetas(blocks: &[Block]) -> Vec<String> {
        blocks
            .iter()
            .map(|block| match block {
                Block::Heading { level, .. } => format!("h{level}"),
                _ => "p".to_owned(),
            })
            .collect()
    }

    fn fixture() -> Doc {
        documento(vec![
            Block::Heading {
                level: 1,
                runs: vec![con("Capítulo ", true, false, false), con("😀 uno", true, false, false)],
            },
            Block::Paragraph {
                runs: vec![
                    plano("Texto "),
                    con("cursiva", false, true, false),
                    plano(" y "),
                    con("subrayado", false, false, true),
                    plano(" fin."),
                ],
            },
            Block::Paragraph { runs: vec![plano("")] },
            Block::ListItem { depth: 0, runs: vec![plano("• Elemento ñ")] },
            Block::Quote { runs: vec![plano("Cita \"中\" \\ {x}")] },
            Block::Paragraph { runs: vec![plano("Último ☃")] },
        ])
    }

    #[test]
    fn texto_une_bloques_con_crlf_sin_salto_final() {
        let doc = documento(vec![
            Block::Paragraph { runs: vec![plano("uno "), plano("ñ")] },
            Block::Heading { level: 1, runs: vec![plano("dos")] },
        ]);
        assert_eq!(plain(&doc), "uno ñ\r\ndos");
    }

    #[test]
    fn texto_omite_las_imagenes() {
        let doc = documento(vec![
            Block::Paragraph { runs: vec![plano("a")] },
            Block::Image { id: 1 },
            Block::Paragraph { runs: vec![plano("b")] },
        ]);
        assert_eq!(plain(&doc), "a\r\nb");
    }

    #[test]
    fn rtf_omite_las_imagenes() {
        let doc = documento(vec![
            Block::Paragraph { runs: vec![plano("a")] },
            Block::Image { id: 1 },
            Block::Paragraph { runs: vec![plano("b")] },
        ]);
        let esperado = [
            CABECERA,
            r"\pard\plain a\par",
            "\n",
            r"\pard\plain b\par",
            "\n}",
        ]
        .concat();
        assert_eq!(rtf(&doc), esperado);
    }

    #[test]
    fn rtf_escapa_caracteres_especiales() {
        let doc = documento(vec![Block::Paragraph {
            runs: vec![plano("\\{}\t\u{a0}\u{1}ñ中😀")],
        }]);
        let esperado = [
            CABECERA,
            r"\pard\plain \\\{\}\tab \~\u1?\u241?\u20013?\u-10179?\u-8704?\par",
            "\n}",
        ]
        .concat();
        assert_eq!(rtf(&doc), esperado);
    }

    #[test]
    fn rtf_da_formato_a_cada_bloque() {
        let doc = documento(vec![
            Block::Heading { level: 2, runs: vec![plano("T")] },
            Block::ListItem { depth: 1, runs: vec![plano("• Uno")] },
            Block::Quote { runs: vec![con("c", false, true, true)] },
        ]);
        let esperado = [
            CABECERA,
            r"\pard\plain\outlinelevel1\b T\par",
            "\n",
            "\\pard\\plain\\li720 \\u8226? Uno\\par",
            "\n",
            r"\pard\plain\li720\ri720 {\i\ul c}\par",
            "\n}",
        ]
        .concat();
        assert_eq!(rtf(&doc), esperado);
    }

    #[test]
    fn ida_y_vuelta_conserva_texto_estilos_y_niveles() {
        let original = fixture();
        let reimportado = parse(rtf(&original).as_bytes());
        assert_eq!(textos(&reimportado), textos(&original.blocks));
        assert_eq!(estilos(&reimportado), estilos(&original.blocks));
        assert_eq!(etiquetas(&reimportado), ["h1", "p", "p", "p", "p", "p"]);
    }
}
