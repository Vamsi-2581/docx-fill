//! Characters XML 1.0 cannot hold must never reach a filled part, whichever way a value is printed.

mod common;

use std::borrow::Cow;

use common::{cell, document, fill, paragraph, row, table, text};
use docx_fill::{FillOptions, FilledDocument, PackageTemplate, fill_part};
use serde_json::{Value, json};

const TYPES: &str = r#"<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"></Types>"#;
const RELS: &str = r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"></Relationships>"#;

fn is_xml_char(c: char) -> bool {
    matches!(c, '\t' | '\n' | '\r') || (c >= ' ' && !matches!(c, '\u{fffe}' | '\u{ffff}'))
}

/// Every character XML 1.0 refuses that a JSON string can hold, in text with a kept letter between.
fn invalid_text() -> String {
    let mut out = String::from("start");
    for c in (0u32..0x20).chain([0xfffe, 0xffff]).filter_map(char::from_u32).filter(|c| !is_xml_char(*c)) {
        out.push(c);
        out.push('.');
    }
    out.push_str("end");
    out
}

fn kept_text() -> String {
    format!("start{}end", ".".repeat(invalid_text().chars().filter(|c| *c == '.').count()))
}

fn assert_all_xml_chars(xml: &str, case: &str) {
    if let Some(bad) = xml.chars().find(|c| !is_xml_char(*c)) {
        panic!("{case}: U+{:04X} in {xml}", bad as u32);
    }
}

fn package(body: &str) -> Vec<(String, Vec<u8>)> {
    vec![
        ("[Content_Types].xml".to_string(), TYPES.as_bytes().to_vec()),
        ("word/_rels/document.xml.rels".to_string(), RELS.as_bytes().to_vec()),
        ("word/document.xml".to_string(), document(body).into_bytes()),
        ("word/settings.xml".to_string(), br#"<w:settings xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"></w:settings>"#.to_vec()),
    ]
}

fn assert_package_xml_chars(parts: &[(String, Cow<[u8]>)], case: &str) {
    for (name, bytes) in parts {
        assert_all_xml_chars(&String::from_utf8_lossy(bytes), &format!("{case}, {name}"));
    }
}

#[test]
fn drops_them_from_a_value_and_keeps_the_rest() {
    let body = fill(&paragraph(&["{v}"]), json!({ "v": invalid_text() }));

    assert_all_xml_chars(&body, "value");
    assert_eq!(text(&body), kept_text());
}

#[test]
fn keeps_tab_and_newline_and_non_ascii_text() {
    let options = FillOptions { linebreaks: false, ..FillOptions::core() };
    let filled = fill_part(&document(&paragraph(&["{v}"])), &json!({ "v": "a\tb\nc\rd é 日本 🎉 \u{fffd}" }), &options).unwrap();

    assert!(filled.contains("a\tb\nc\rd é 日本 🎉 \u{fffd}"), "{filled}");
}

#[test]
fn drops_them_with_line_breaks_on() {
    let body = fill(&paragraph(&["{v}"]), json!({ "v": "a\u{b}\nb\u{1}" }));

    assert_all_xml_chars(&body, "line breaks");
    assert_eq!(text(&body), "ab");
    assert!(body.contains("<w:br/>"), "{body}");
}

#[test]
fn drops_them_from_every_tag_that_prints_a_value() {
    let value = invalid_text();
    let cases: [(&str, String, Value); 9] = [
        ("loop item", paragraph(&["{#l}{.}{/l}"]), json!({ "l": [value] })),
        ("section string", paragraph(&["{#v}{.}{/v}"]), json!({ "v": value })),
        ("placeholder", paragraph(&["{?v}"]), json!({ "v": value })),
        ("list placeholder", paragraph(&["{?l}"]), json!({ "l": [value] })),
        ("footnote", paragraph(&["{:footnote v}"]), json!({ "v": value })),
        ("html", paragraph(&["{~~v}"]), json!({ "v": value })),
        ("inline html", paragraph(&["a{~v}b"]), json!({ "v": value })),
        ("full table", paragraph(&["{:table t}"]), json!({ "t": { "header": [value], "data": [[value, { "text": value }]] } })),
        ("grid", format!("{}{}{}", paragraph(&["{:#grid t}"]), paragraph(&["{v}"]), paragraph(&["{:/grid}"])), json!({ "t": { "size": [1, 1], "data": [{ "v": value }] } })),
    ];
    for (name, template, data) in cases {
        assert_all_xml_chars(&fill(&template, data), name);
    }
}

#[test]
fn drops_character_references_html_decodes_to_them() {
    for html in ["<p>a&#1;b</p>", "<p>a&#0;b</p>", "<p>a&#xB;b</p>", "<p>a&#xFFFF;b</p>", "<p>a&#xFFFE;b</p>", "a\u{1}<b>b\u{b}</b>"] {
        let body = fill(&paragraph(&["{~~v}"]), json!({ "v": html }));

        assert_all_xml_chars(&body, html);
        assert_eq!(text(&body), "ab", "{html}");
    }
}

#[test]
fn drops_them_from_html_attributes() {
    let html = "<p style=\"font-family:'A\u{1}B';color:#f00\"><a href=\"http://x.test/\u{1}\">l</a> <a href=\"#a\u{b}b\">m</a></p>";

    let parts = PackageTemplate::parse(package(&paragraph(&["{~~v}"])), &FillOptions::default()).unwrap();

    assert_package_xml_chars(&parts.render(&json!({ "v": html })), "html attributes");
}

#[test]
fn drops_them_from_style_values() {
    let value = invalid_text();
    let style = json!({ "pStyle": value, "fontFamily": value, "textColor": value, "cellBackground": value, "bulletColor": value, "fillColor": value, "strokeColor": value, "textAlign": value, "underline": value, "borders": { "top": { "val": value, "color": value, "sz": value } } });
    let template = table(&[row(&[cell(&["{:stylecell s}{:stylerow s}{:stylepar s}{:stylerun s}{:stylestart s}x{:/style}"])])]);

    assert_all_xml_chars(&fill(&template, json!({ "s": style })), "style");
}

#[test]
fn drops_them_from_comments_and_their_authors() {
    let value = invalid_text();
    let data = json!({ "v": { "text": value, "comments": [{ "comment": value, "author": value, "date": value }] } });
    let template = PackageTemplate::parse(package(&paragraph(&["{v}"])), &FillOptions::default()).unwrap();

    assert_package_xml_chars(&template.render(&data), "comment");
}

#[test]
fn drops_them_from_a_page_option() {
    let value = invalid_text();
    let page = format!("{}{}", paragraph(&["{:page p}"]), paragraph(&["x"]));
    let parts = PackageTemplate::parse(package(&page), &FillOptions::default()).unwrap();
    assert_package_xml_chars(&parts.render(&json!({ "p": { "orientation": value, "size": value } })), "page");
}

#[test]
fn drops_them_from_document_edits_and_text_edits() {
    let value = invalid_text();
    let options = FillOptions::default().read_json(&json!({ "comments": { "author": value }, "document": { "language": value, "watermark": { "text": value, "diagonal": true } } })).unwrap();
    let template = PackageTemplate::parse(package(&paragraph(&["hello"])), &options).unwrap();
    let filled: Vec<(String, Vec<u8>)> = template.render(&json!({})).into_iter().map(|(name, bytes)| (name, bytes.into_owned())).collect();
    assert_package_xml_chars(&filled.iter().map(|(name, bytes)| (name.clone(), Cow::Borrowed(bytes.as_slice()))).collect::<Vec<_>>(), "document edits");

    let mut document = FilledDocument::new(filled);
    let replaced = document.replace_all("hello", &value);
    assert_eq!(replaced, 1);
    let edits = json!([{ "op": "addComment", "find": "start", "comment": value, "author": value, "initials": value, "date": value }]);
    document.run_json(&edits).unwrap();

    let parts = document.into_parts();
    assert_package_xml_chars(&parts.iter().map(|(name, bytes)| (name.clone(), Cow::Borrowed(bytes.as_slice()))).collect::<Vec<_>>(), "text edits");
}
