use std::sync::Arc;

use serde::Serialize;
use ts_rs::TS;

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq, TS)]
#[ts(export)]
pub enum Composition {
    Text,
    Book,
}

#[derive(Serialize, Clone, Debug, TS)]
#[ts(export)]
pub struct Doc {
    pub title: String,
    pub initial_char_index: u32,
    pub composition: Composition,
    pub chapters: Vec<u32>,
    pub blocks: Vec<Block>,
    #[serde(skip)]
    #[ts(skip)]
    pub images: Vec<Image>,
}

#[derive(Clone, Debug)]
pub struct Image {
    pub id: u32,
    pub mime: &'static str,
    pub bytes: Arc<[u8]>,
}

impl Doc {
    pub fn flat_len(&self) -> u32 {
        let unidades: u32 = self
            .blocks
            .iter()
            .map(|block| {
                block
                    .runs()
                    .iter()
                    .map(|run| run.text.encode_utf16().count() as u32)
                    .sum::<u32>()
            })
            .sum();
        unidades + (self.blocks.len() as u32).saturating_sub(1)
    }
}

#[derive(Serialize, Clone, Debug, TS)]
#[ts(export)]
#[serde(tag = "type")]
pub enum Block {
    Paragraph { runs: Vec<Run> },
    Heading { level: u8, runs: Vec<Run> },
    ListItem { depth: u8, runs: Vec<Run> },
    Quote { runs: Vec<Run> },
    Image { id: u32 },
}

impl Block {
    pub fn runs(&self) -> &[Run] {
        match self {
            Self::Paragraph { runs }
            | Self::Heading { runs, .. }
            | Self::ListItem { runs, .. }
            | Self::Quote { runs } => runs,
            Self::Image { .. } => &[],
        }
    }
}

#[derive(Serialize, Clone, Debug, TS)]
#[ts(export)]
pub struct Run {
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub bold: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub italic: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub underline: Option<bool>,
}

impl Run {
    pub fn plain(text: String) -> Run {
        Run {
            text,
            bold: None,
            italic: None,
            underline: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flat_len_cuenta_las_cuatro_variantes() {
        let doc = Doc {
            title: "t".into(),
            initial_char_index: 0,
            composition: Composition::Book,
            chapters: Vec::new(),
            blocks: vec![
                Block::Paragraph { runs: vec![Run::plain("ab".into())] },
                Block::Heading { level: 1, runs: vec![Run::plain("cde".into())] },
                Block::ListItem { depth: 0, runs: vec![Run::plain("f".into())] },
                Block::Quote {
                    runs: vec![Run::plain("g".into()), Run::plain("𝄞".into())],
                },
            ],
            images: Vec::new(),
        };
        // 2 + 3 + 1 + (1 + 2 unidades UTF-16) = 9, más 3 separadores.
        assert_eq!(doc.flat_len(), 12);
    }

    #[test]
    fn flat_len_imagen_no_suma_unidades() {
        let doc = Doc {
            title: "t".into(),
            initial_char_index: 0,
            composition: Composition::Book,
            chapters: Vec::new(),
            blocks: vec![
                Block::Paragraph { runs: vec![Run::plain("ab".into())] },
                Block::Image { id: 1 },
                Block::Paragraph { runs: vec![Run::plain("c".into())] },
            ],
            images: Vec::new(),
        };
        // 2 + 0 + 1 = 3, más 2 separadores.
        assert_eq!(doc.flat_len(), 5);
    }
}
