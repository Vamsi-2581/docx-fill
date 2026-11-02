//! Which parts of a package hold tags, read from their content types.

use std::collections::BTreeMap;

use crate::xml::{ElementKind, Token, read_attribute, tokenize};
use crate::{DOCUMENT_PART, is_template_part};

const MAIN_TYPES: [&str; 4] = [
    "application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml",
    "application/vnd.openxmlformats-officedocument.wordprocessingml.template.main+xml",
    "application/vnd.ms-word.document.macroEnabled.main+xml",
    "application/vnd.ms-word.template.macroEnabledTemplate.main+xml",
];
const STORY_TYPES: [&str; 4] = [
    "application/vnd.openxmlformats-officedocument.wordprocessingml.header+xml",
    "application/vnd.openxmlformats-officedocument.wordprocessingml.footer+xml",
    "application/vnd.openxmlformats-officedocument.wordprocessingml.footnotes+xml",
    "application/vnd.openxmlformats-officedocument.wordprocessingml.comments+xml",
];
const PROPERTIES_TYPES: [&str; 3] = [
    "application/vnd.openxmlformats-package.core-properties+xml",
    "application/vnd.openxmlformats-officedocument.extended-properties+xml",
    "application/vnd.openxmlformats-officedocument.custom-properties+xml",
];

/// What a part holds, as far as filling goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PartKind {
    /// The main document's body.
    Main,
    /// Paragraphs outside the body: a header, footer, footnotes or comments.
    Story,
    /// Document properties, whose tags sit in plain element text.
    Properties,
    Other,
}

/// Each part's kind, by its override in `[Content_Types].xml`. A part without one is the body,
/// a header or a footer by name: `word/document.xml`, `word/headerN.xml` or `word/footerN.xml`.
pub(crate) struct PartKinds {
    overrides: BTreeMap<String, PartKind>,
}

impl PartKinds {
    pub(crate) fn read(content_types: Option<&str>) -> PartKinds {
        let tokens = content_types.and_then(|xml| tokenize(xml).ok()).unwrap_or_default();
        let overrides = tokens
                .iter()
                .filter_map(|token| match token {
                    Token::Element { kind: ElementKind::Open | ElementKind::SelfClosing, name: "Override", raw } => Some(*raw),
                    _ => None,
                })
                .filter_map(|raw| Some((read_attribute(raw, "PartName")?.trim_start_matches('/').to_string(), read_kind(read_attribute(raw, "ContentType")?))))
                .collect();
        PartKinds { overrides }
    }

    pub(crate) fn kind(&self, name: &str) -> PartKind {
        match self.overrides.get(name) {
            Some(kind) => *kind,
            None if name == DOCUMENT_PART => PartKind::Main,
            None if is_template_part(name) => PartKind::Story,
            None => PartKind::Other,
        }
    }
}

fn read_kind(content_type: &str) -> PartKind {
    if MAIN_TYPES.contains(&content_type) {
        PartKind::Main
    } else if STORY_TYPES.contains(&content_type) {
        PartKind::Story
    } else if PROPERTIES_TYPES.contains(&content_type) {
        PartKind::Properties
    } else {
        PartKind::Other
    }
}
