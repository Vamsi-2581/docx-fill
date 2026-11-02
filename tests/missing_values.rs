//! `FillHooks::missing_value` decides what a missing or null value prints, as `nullGetter` does.

mod common;

use common::{document, paragraph, paragraph_texts};
use docx_fill::{FillHooks, FillOptions, MissingKind, MissingValue, ModuleTag, Template};
use serde_json::{Value, json};

#[derive(Default)]
struct Hooks {
    missing: Vec<(MissingKind, String)>,
    module_values: Vec<Option<Value>>,
    reply: Option<fn(&MissingValue) -> String>,
}

impl FillHooks for Hooks {
    fn missing_value(&mut self, _part: &str, tag: MissingValue) -> Option<String> {
        self.missing.push((tag.kind, tag.name.to_string()));
        self.reply.map(|reply| reply(&tag))
    }

    fn module_tag(&mut self, _part: &str, tag: ModuleTag) -> String {
        self.module_values.push(tag.value.cloned());
        String::new()
    }
}

fn fill(body: &str, data: Value, hooks: &mut Hooks) -> String {
    let options = FillOptions { module_prefixes: vec!["$".into()], ..FillOptions::default() };
    Template::parse(&document(body), &options).unwrap().render_with(&data, hooks)
}

#[test]
fn keeps_a_missing_tag_as_typed() {
    let mut hooks = Hooks { reply: Some(|tag| format!("{{{}}}", tag.written)), ..Hooks::default() };
    let xml = fill(&paragraph(&["Dear {name}, {#l}{x}{/l}"]), json!({ "l": [{ "x": null }] }), &mut hooks);

    assert_eq!(paragraph_texts(&xml), ["Dear {name}, {x}"]);
}

#[test]
fn collects_the_names_of_missing_values() {
    let mut hooks = Hooks::default();
    let body = paragraph(&["{a}{b}{c}"]) + &paragraph(&["{@raw}"]) + &paragraph(&["{$module}"]);
    let xml = fill(&body, json!({ "b": "B", "c": null }), &mut hooks);

    assert_eq!(paragraph_texts(&xml), ["B", ""]);
    assert_eq!(hooks.missing, [(MissingKind::Value, "a".to_string()), (MissingKind::Value, "c".to_string()), (MissingKind::Raw, "raw".to_string())]);
    assert_eq!(hooks.module_values, [None]);
}

#[test]
fn prints_a_missing_raw_tags_text_as_xml() {
    let mut hooks = Hooks { reply: Some(|_| "<w:p><w:r><w:t>none</w:t></w:r></w:p>".to_string()), ..Hooks::default() };
    let xml = fill(&paragraph(&["{@raw}"]), json!({}), &mut hooks);

    assert_eq!(paragraph_texts(&xml), ["none"]);
}

#[test]
fn escapes_a_missing_values_text() {
    let mut hooks = Hooks { reply: Some(|_| "<none>".to_string()), ..Hooks::default() };
    let xml = fill(&paragraph(&["{a}"]), json!({}), &mut hooks);

    assert!(xml.contains("&lt;none&gt;"), "{xml}");
}

#[test]
fn leaves_shown_values_and_other_raw_values_alone() {
    let mut hooks = Hooks::default();
    fill(&(paragraph(&["{a}{b}"]) + &paragraph(&["{@n}"])), json!({ "a": 0, "b": "", "n": 3 }), &mut hooks);

    assert!(hooks.missing.is_empty());
}
