//! Delimiters other than `{ }`, as a signatures pass uses `[[ ]]`.

mod common;

use common::{document, paragraph, text};
use docx_fill::{Delimiters, FillOptions, fill_part};
use serde_json::json;

fn fill_with(body: &str, start: &str, end: &str) -> Result<String, String> {
    let options = FillOptions { delimiters: Delimiters { start: start.into(), end: end.into() }, ..FillOptions::default() };
    fill_part(&document(body), &json!({ "name": "N", "rows": [{ "x": 1 }, { "x": 2 }] }), &options).map(|xml| text(&xml)).map_err(|error| error.message)
}

#[test]
fn fills_tags_between_two_character_delimiters() {
    assert_eq!(fill_with(&paragraph(&["[[name]] [[#rows]][[x]][[/rows]]"]), "[[", "]]").unwrap(), "N 12");
}

#[test]
fn leaves_the_other_delimiters_as_text() {
    assert_eq!(fill_with(&paragraph(&["{name} [ x ] [[na", "me]]"]), "[[", "]]").unwrap(), "{name} [ x ] N");
}

#[test]
fn rejects_an_unclosed_tag() {
    assert_eq!(fill_with(&paragraph(&["[[name"]), "[[", "]]").unwrap_err(), r#"The tag beginning with "[[name" is unclosed"#);
}

#[test]
fn fills_tags_between_doubled_braces() {
    assert_eq!(fill_with(&paragraph(&["{{name}}"]), "{{", "}}").unwrap(), "N");
}

#[test]
fn fills_tags_when_start_and_end_delimiter_are_the_same() {
    assert_eq!(fill_with(&paragraph(&["%name%"]), "%", "%").unwrap(), "N");
}

#[test]
fn fills_several_same_delimiter_tags_in_one_paragraph() {
    let options = FillOptions { delimiters: Delimiters { start: "%".into(), end: "%".into() }, ..FillOptions::default() };
    let xml = fill_part(&document(&paragraph(&["%a% and %b%"])), &json!({ "a": "one", "b": "two" }), &options)
        .map(|xml| text(&xml))
        .unwrap();
    assert_eq!(xml, "one and two");
}

#[test]
fn fills_tags_between_delimiters_xml_escapes() {
    for (start, end) in [("<<", ">>"), ("<%", "%>"), ("&", "&")] {
        let written = format!("{start}name{end}").replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;");
        assert_eq!(fill_with(&paragraph(&[&written]), start, end).unwrap(), "N", "{start}name{end}");
    }
}

#[test]
fn fills_a_greater_than_delimiter_written_raw() {
    assert_eq!(fill_with(&paragraph(&["&lt;&lt;name>>"]), "<<", ">>").unwrap(), "N");
}

#[test]
fn names_a_doubled_open_delimiter() {
    assert_eq!(fill_with(&paragraph(&["x {{name} y"]), "{", "}").unwrap_err(), r#"The tag beginning with "{{name" has duplicate open tags"#);
}

#[test]
fn names_a_doubled_close_delimiter() {
    assert_eq!(fill_with(&paragraph(&["x {name}} y"]), "{", "}").unwrap_err(), r#"The tag ending with "name}}" has duplicate close tags"#);
}

#[test]
fn keeps_unclosed_and_unopened_apart_from_doubled_delimiters() {
    assert_eq!(fill_with(&paragraph(&["{a {b}"]), "{", "}").unwrap_err(), r#"The tag beginning with "{a " is unclosed"#);
    assert_eq!(fill_with(&paragraph(&["{a} b}"]), "{", "}").unwrap_err(), r#"The tag beginning with " b" is unopened"#);
}

#[test]
fn keeps_escaped_text_apart_from_an_ampersand_delimiter() {
    assert_eq!(fill_with(&paragraph(&["a &lt; b &amp;name&amp;"]), "&", "&").unwrap(), "a &lt; b N");
}

#[test]
fn names_a_doubled_open_delimiter_before_non_ascii_text() {
    for (template, context) in [("{{prénom}", "{{prénom"), ("é{{日本}", "{{日本"), ("{{🎉", "{{🎉")] {
        let error = fill_part(&document(&paragraph(&[template])), &json!({}), &FillOptions::core()).unwrap_err();

        assert_eq!(error.message, format!("The tag beginning with \"{context}\" has duplicate open tags"));
    }
}
