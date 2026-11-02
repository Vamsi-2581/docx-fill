//! Restores what a filled part must hold after loops and block tags removed content. From the
//! WordprocessingML spec: a table needs a row and a row needs a cell, or they go, and a cell,
//! header, footer or text box must end in a paragraph.

use crate::xml::{ElementKind, Token, tokenize};

const TABLE: &str = "w:tbl";
const ROW: &str = "w:tr";
const CELL: &str = "w:tc";
const PARAGRAPH: &str = "w:p";
// Content controls and custom XML can hold a row, a cell or a paragraph.
const WRAPPERS: [&str; 2] = ["w:sdt", "w:customXml"];
const ENDS_IN_PARAGRAPH: [&str; 7] = [CELL, "w:hdr", "w:ftr", "w:txbxContent", "w:footnote", "w:endnote", "w:comment"];
const EMPTY_PARAGRAPH: &str = "<w:p/>";

/// An element open in the output.
struct Frame<'a> {
    name: &'a str,
    start: usize,
    /// A table's row or a row's cell.
    has_required_child: bool,
    last_block: Option<&'a str>,
}

impl<'a> Frame<'a> {
    fn add_child(&mut self, name: &'a str) {
        let is_wrapper = WRAPPERS.contains(&name);
        self.has_required_child |= is_wrapper || (self.name == TABLE && name == ROW) || (self.name == ROW && name == CELL);
        if is_wrapper || name == PARAGRAPH || name == TABLE {
            self.last_block = Some(name);
        }
    }

    fn needs_paragraph(&self) -> bool {
        ENDS_IN_PARAGRAPH.contains(&self.name) && matches!(self.last_block, None | Some(TABLE))
    }
}

/// `xml` with its tables, cells and other containers made valid. `None` when nothing changed or
/// the XML is not balanced, as a raw tag's value can make it.
pub(crate) fn repair_structure(xml: &str) -> Option<String> {
    let tokens = tokenize(xml).ok()?;
    let mut out = String::with_capacity(xml.len());
    let mut open: Vec<Frame> = Vec::new();
    let mut is_changed = false;
    for token in tokens {
        let (kind, name, raw) = match token {
            Token::Text(text) => {
                out.push_str(text);
                continue;
            }
            Token::Element { kind, name, raw } => (kind, name, raw),
        };
        match kind {
            ElementKind::Open => {
                open.push(Frame { name, start: out.len(), has_required_child: false, last_block: None });
                out.push_str(raw);
            }
            ElementKind::SelfClosing => {
                if let Some(parent) = open.last_mut() {
                    parent.add_child(name);
                }
                out.push_str(raw);
            }
            ElementKind::Close => {
                let frame = open.pop().filter(|frame| frame.name == name)?;
                if (name == TABLE || name == ROW) && !frame.has_required_child {
                    out.truncate(frame.start);
                    is_changed = true;
                    continue;
                }
                if frame.needs_paragraph() {
                    out.push_str(EMPTY_PARAGRAPH);
                    is_changed = true;
                }
                out.push_str(raw);
                if let Some(parent) = open.last_mut() {
                    parent.add_child(name);
                }
            }
            ElementKind::Other => out.push_str(raw),
        }
    }
    (is_changed && open.is_empty()).then_some(out)
}
