//! Page breaks in a paragraph loop's tag paragraphs, as black-box runs of docxtemplater core
//! 3.71.1 place them.

mod common;

use common::fill;
use serde_json::json;

const BREAK: &str = r#"<w:r><w:br w:type="page"/></w:r>"#;

fn plain(text: &str) -> String {
    format!(r#"<w:p><w:r><w:t xml:space="preserve">{text}</w:t></w:r></w:p>"#)
}

fn items(count: usize) -> serde_json::Value {
    json!({ "l": vec![json!({}); count] })
}

fn with_break(tag: &str, is_before: bool) -> String {
    let tag = format!("<w:r><w:t>{tag}</w:t></w:r>");
    if is_before { format!("<w:p>{BREAK}{tag}</w:p>") } else { format!("<w:p>{tag}{BREAK}</w:p>") }
}

#[test]
fn moves_a_closing_paragraphs_break_to_the_end_of_the_last_item() {
    for is_before in [true, false] {
        let template = plain("{#l}") + &plain("x") + &with_break("{/l}", is_before);
        let last = format!(r#"<w:p><w:r><w:t xml:space="preserve">x</w:t></w:r>{BREAK}</w:p>"#);
        assert_eq!(fill(&template, items(2)), plain("x") + &last);
        assert_eq!(fill(&template, items(1)), last);
        assert_eq!(fill(&template, items(0)), format!("<w:p>{BREAK}</w:p>"));
    }
}

#[test]
fn moves_a_closing_paragraphs_break_after_the_items_last_paragraph() {
    let template = plain("{#l}") + &plain("x") + &plain("y") + &with_break("{/l}", true);
    let last = format!(r#"<w:p><w:r><w:t xml:space="preserve">y</w:t></w:r>{BREAK}</w:p>"#);
    assert_eq!(fill(&template, items(1)), plain("x") + &last);
}

#[test]
fn keeps_an_opening_paragraphs_break_before_each_item() {
    for is_before in [true, false] {
        let template = with_break("{#l}", is_before) + &plain("x") + &plain("{/l}");
        let item = format!("<w:p>{BREAK}</w:p>") + &plain("x");
        assert_eq!(fill(&template, items(2)), item.repeat(2));
        assert_eq!(fill(&template, items(1)), item);
        assert_eq!(fill(&template, items(0)), "");
    }
}
