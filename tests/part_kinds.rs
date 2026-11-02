//! Template parts are found by content type, as docxtemplater core 3.71.1 finds them.

use docx_fill::{FillOptions, PackageTemplate};
use serde_json::json;

const W: &str = r#"xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main""#;
const WORD: &str = "application/vnd.openxmlformats-officedocument.wordprocessingml";

fn tagged(root: &str) -> String {
    format!(r#"<{root} {W}><w:p><w:r><w:t>{{name}}</w:t></w:r></w:p></{root}>"#)
}

fn fill(parts: &[(&str, String, Option<String>)], options: &FillOptions) -> Vec<(String, String)> {
    let overrides: String = parts.iter().filter_map(|(name, _, kind)| kind.as_ref().map(|kind| format!(r#"<Override PartName="/{name}" ContentType="{kind}"/>"#))).collect();
    let mut package = vec![("[Content_Types].xml".to_string(), format!("<Types>{overrides}</Types>").into_bytes())];
    package.extend(parts.iter().map(|(name, xml, _)| (name.to_string(), xml.clone().into_bytes())));
    let template = PackageTemplate::parse(package, options).unwrap();
    template.render(&json!({ "name": "V&W" })).into_iter().map(|(name, bytes)| (name, String::from_utf8(bytes.into_owned()).unwrap())).collect()
}

fn part<'a>(parts: &'a [(String, String)], name: &str) -> &'a str {
    &parts.iter().find(|(each, _)| each == name).unwrap().1
}

fn word(kind: &str) -> Option<String> {
    Some(format!("{WORD}.{kind}+xml"))
}

#[test]
fn fills_headers_footers_footnotes_and_comments_by_content_type() {
    let parts = fill(
        &[
            ("word/document.xml", tagged("w:document"), word("document.main")),
            ("word/header_first.xml", tagged("w:hdr"), word("header")),
            ("word/footerDefault.xml", tagged("w:ftr"), word("footer")),
            ("word/footnotes.xml", tagged("w:footnotes"), word("footnotes")),
            ("word/comments.xml", tagged("w:comments"), word("comments")),
            ("word/endnotes.xml", tagged("w:endnotes"), word("endnotes")),
        ],
        &FillOptions::default(),
    );

    for name in ["word/document.xml", "word/header_first.xml", "word/footerDefault.xml", "word/footnotes.xml", "word/comments.xml"] {
        assert!(part(&parts, name).contains("V&amp;W"), "{name}");
    }
    assert!(part(&parts, "word/endnotes.xml").contains("{name}"));
}

#[test]
fn fills_a_main_document_under_another_name() {
    let parts = fill(&[("word/document2.xml", tagged("w:document"), word("document.main"))], &FillOptions::default());

    assert!(part(&parts, "word/document2.xml").contains("V&amp;W"));
}

#[test]
fn fills_document_properties_unless_turned_off() {
    let core = r#"<cp:coreProperties xmlns:cp="c" xmlns:dc="d"><dc:title>{name}</dc:title><dc:creator>Me</dc:creator></cp:coreProperties>"#.to_string();
    let parts_of = |options: &FillOptions| {
        fill(
            &[
                ("word/document.xml", tagged("w:document"), word("document.main")),
                ("docProps/core.xml", core.clone(), Some("application/vnd.openxmlformats-package.core-properties+xml".to_string())),
            ],
            options,
        )
    };

    let filled = parts_of(&FillOptions::default());
    assert_eq!(part(&filled, "docProps/core.xml"), r#"<cp:coreProperties xmlns:cp="c" xmlns:dc="d"><dc:title>V&amp;W</dc:title><dc:creator>Me</dc:creator></cp:coreProperties>"#);

    let left = parts_of(&FillOptions { fill_properties: false, ..FillOptions::default() });
    assert_eq!(part(&left, "docProps/core.xml"), core);
}
