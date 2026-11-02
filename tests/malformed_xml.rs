//! XML that does not balance is refused when parsing, never a panic while filling.

mod common;

use common::{document, paragraph};
use docx_fill::{FillOptions, fill_part};
use serde_json::json;

fn parse_error(body: &str) -> String {
    fill_part(&document(body), &json!({}), &FillOptions::default()).unwrap_err().message
}

#[test]
fn refuses_a_close_tag_that_does_not_match() {
    assert_eq!(parse_error("<w:p><w:r><w:t>x</w:t></w:p></w:r>"), "The XML is malformed: </w:p> closes <w:r>");
}

#[test]
fn refuses_an_element_left_open() {
    assert_eq!(parse_error("<w:p><w:r><w:t>x</w:t></w:r>"), "The XML is malformed: </w:body> closes <w:p>");
    let unclosed = fill_part("<w:document><w:body><w:p/>", &json!({}), &FillOptions::default()).unwrap_err();
    assert_eq!(unclosed.message, "The XML is malformed: <w:body> is never closed");
}

#[test]
fn refuses_the_reported_paragraph_loop() {
    let body = "<w:p><w:r><w:t>{#l}</w:t></w:r</w:p>/w:p><w:p><w:r><w:t>x</w:t></w:r></w:p><w:p><w:r><w:t>{/l}</w:t></w:r></w:p>:t></w:<w:tc>r></w:p>";
    assert!(fill_part(&document(body), &json!({ "l": [1, 2] }), &FillOptions::default()).is_err());
}

#[test]
fn fills_or_refuses_mutated_templates_without_panicking() {
    let template = paragraph(&["{#l}"]) + &paragraph(&["x {y}"]) + "<w:tbl><w:tr><w:tc>" + &paragraph(&["{#r}{z}{/r}"]) + "</w:tc></w:tr></w:tbl>" + &paragraph(&["{/l}"]);
    let template = document(&template);
    let data = json!({ "l": [{ "y": 1, "r": [{ "z": 2 }, { "z": 3 }] }, {}] });
    // xorshift, seeded so a failure repeats.
    let mut state: u64 = 0x2545_F491_4F6C_DD1D;
    let mut next = |bound: usize| {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        (state % bound as u64) as usize
    };
    for _ in 0..3000 {
        let mut bytes = template.clone().into_bytes();
        for _ in 0..1 + next(3) {
            let start = next(bytes.len());
            let end = (start + 1 + next(12)).min(bytes.len());
            let piece: Vec<u8> = bytes[start..end].to_vec();
            // Drop the piece, or copy it to a random place.
            if next(2) == 0 {
                bytes.drain(start..end);
            } else {
                let at = next(bytes.len());
                bytes.splice(at..at, piece);
            }
        }
        let Ok(mutated) = String::from_utf8(bytes) else { continue };
        let _ = fill_part(&mutated, &data, &FillOptions::default());
    }
}
