//! Empty data must leave tables, cells and other containers as the WordprocessingML spec requires.
//! Removing tables follows black-box runs of docxtemplater core 3.71.1.

mod common;

use std::borrow::Cow;

use common::{cell, document, elements, fill, paragraph, read_rows, row, table, text};
use docx_fill::{FillOptions, PackageTemplate, Subtemplate, fill_part};
use serde_json::{Value, json};

const GRID: &str = r#"<w:tblPr/><w:tblGrid><w:gridCol w:w="1"/></w:tblGrid>"#;
const SECTION: &str = r#"<w:sectPr><w:pgSz w:w="1" w:h="2"/></w:sectPr>"#;
const WORD_NAMESPACE: &str = r#"xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main""#;

/// `table` with the properties and grid Word writes.
fn grid_table(rows: &[String]) -> String {
    format!("<w:tbl>{GRID}{}</w:tbl>", rows.concat())
}

fn cell_of(content: &str) -> String {
    format!("<w:tc>{content}</w:tc>")
}

fn only_row_loop() -> String {
    grid_table(&[row(&[cell(&["{#l}{.}"]), cell(&["{/l}"])])])
}

/// A one-cell table holding `content`, with a paragraph after it.
fn in_cell(content: &str) -> String {
    format!("{}{}", grid_table(&[row(&[cell_of(content)])]), paragraph(&["after"]))
}

fn assert_cell_ends_in_paragraph(body: &str, data: &str) {
    let cells = elements(body, "w:tc");
    assert_eq!(cells.len(), 1, "{data}: {body}");
    assert!(cells[0].ends_with("</w:p></w:tc>") || cells[0].ends_with("<w:p/></w:tc>"), "{data}: {body}");
}

#[test]
fn removes_a_table_whose_only_row_a_loop_removed() {
    let cases: [(&str, Value); 4] = [
        ("empty list", json!({ "l": [] })),
        ("false", json!({ "l": false })),
        ("missing", json!({})),
        ("null", json!({ "l": null })),
    ];
    for (name, data) in cases {
        let body = fill(&format!("{}{}", paragraph(&["a"]), only_row_loop()), data);

        assert!(!body.contains("<w:tbl"), "{name}: {body}");
        assert_eq!(text(&body), "a", "{name}");
    }
}

#[test]
fn keeps_a_table_a_loop_leaves_rows_in() {
    let template = grid_table(&[row(&[cell(&["head"])]), row(&[cell(&["{#l}{.}"]), cell(&["{/l}"])])]);

    assert_eq!(read_rows(&fill(&template, json!({ "l": [] }))), [["head"]]);
    assert_eq!(read_rows(&fill(&template, json!({ "l": [1] }))), [vec!["head".to_string()], vec!["1".to_string(), String::new()]]);
}

#[test]
fn removes_tables_a_dash_loop_emptied() {
    let by_row = grid_table(&[row(&[cell(&["{-w:tr l}{.}{/l}"]), cell(&["z"])])]);
    let by_cell = grid_table(&[row(&[cell(&["{-w:tc l}{.}{/l}"])])]);
    let by_table = grid_table(&[row(&[cell(&["{-w:tbl l}{.}{/l}"])])]);
    for template in [by_row, by_cell, by_table] {
        let body = fill(&format!("{template}{}", paragraph(&["a"])), json!({ "l": [] }));

        assert!(!body.contains("<w:tbl"), "{body}");
        assert_eq!(text(&body), "a");
    }
}

#[test]
fn removes_a_row_whose_only_cell_a_dash_loop_removed_and_keeps_the_other_rows() {
    let template = grid_table(&[row(&[cell(&["keep"])]), row(&[cell(&["{-w:tc l}{.}{/l}"])])]);

    assert_eq!(read_rows(&fill(&template, json!({ "l": [] }))), [["keep"]]);
}

#[test]
fn removes_a_table_a_vertical_loop_emptied() {
    let template = format!("{}{}", grid_table(&[row(&[cell(&["{:vt#c}{v}{:vt/c}"])])]), paragraph(&["a"]));

    let body = fill(&template, json!({ "c": [] }));

    assert!(!body.contains("<w:tbl"), "{body}");
}

#[test]
fn gives_a_cell_a_paragraph_when_a_paragraph_loop_empties_it() {
    let loop_paragraphs = format!("{}{}{}", paragraph(&["{#l}"]), paragraph(&["x"]), paragraph(&["{/l}"]));

    for data in [json!({ "l": [] }), json!({ "l": false }), json!({})] {
        let body = fill(&in_cell(&loop_paragraphs), data.clone());

        assert_cell_ends_in_paragraph(&body, &data.to_string());
    }
}

#[test]
fn gives_a_cell_a_paragraph_when_a_block_tag_prints_nothing() {
    let cases: [(&str, Value); 8] = [
        ("{@x}", json!({ "x": "" })),
        ("{@x}", json!({})),
        ("{?x}", json!({ "x": false })),
        ("{:table x}", json!({})),
        ("{:include x}", json!({})),
        ("{~~x}", json!({})),
        ("{%%x}", json!({})),
        ("{-w:p l}{.}{/l}", json!({ "l": [] })),
    ];
    for (tag, data) in cases {
        let body = fill(&in_cell(&paragraph(&[tag])), data);

        assert_cell_ends_in_paragraph(&body, tag);
    }
}

#[test]
fn gives_a_cell_a_paragraph_when_only_a_segment_is_in_it() {
    let segment = format!("{}{}{}", paragraph(&["{:segment s}"]), paragraph(&["x"]), paragraph(&["{:segment/}"]));

    assert_cell_ends_in_paragraph(&fill(&in_cell(&segment), json!({})), "segment");
}

#[test]
fn adds_a_paragraph_after_a_table_that_ends_a_cell() {
    let inner = table(&[row(&[cell(&["inner"])])]);

    let raw = fill(&in_cell(&paragraph(&["{@x}"])), json!({ "x": inner }));
    assert!(raw.contains("</w:tbl><w:p/></w:tc>"), "{raw}");

    let looped = format!("{}{inner}{}", paragraph(&["{#l}"]), paragraph(&["{/l}"]));
    let printed = fill(&in_cell(&looped), json!({ "l": [1] }));
    assert!(printed.contains("</w:tbl><w:p/></w:tc>"), "{printed}");
}

#[test]
fn removes_a_nested_table_and_keeps_its_cell_valid() {
    let inner = grid_table(&[row(&[cell(&["{#l}{.}"]), cell(&["{/l}"])])]);

    let alone = fill(&in_cell(&inner), json!({ "l": [] }));
    assert_eq!(alone.matches("<w:tbl>").count(), 1, "{alone}");
    assert_cell_ends_in_paragraph(&alone, "alone");

    let beside = fill(&in_cell(&format!("{inner}{}", paragraph(&["keep"]))), json!({ "l": [] }));
    assert_eq!(text(elements(&beside, "w:tc")[0]), "keep");
    assert!(!beside.contains("<w:p/>"), "{beside}");
}

#[test]
fn leaves_a_table_it_does_not_need_to_change_as_it_was() {
    let template = format!("{}{}", grid_table(&[row(&[cell(&["{v}"])])]), paragraph(&["a"]));

    let body = fill(&template, json!({ "v": 1 }));

    assert_eq!(body, format!("{}{}", grid_table(&[row(&[cell_of(r#"<w:p><w:r><w:t xml:space="preserve">1</w:t></w:r></w:p>"#)])]), paragraph(&["a"])));
}

#[test]
fn leaves_a_cell_ending_in_a_content_control_alone() {
    let control = format!("<w:sdt><w:sdtContent>{}</w:sdtContent></w:sdt>", paragraph(&["x"]));

    let body = fill(&in_cell(&control), json!({}));

    assert!(body.contains("</w:sdt></w:tc>"), "{body}");
}

#[test]
fn leaves_a_row_of_content_controls_alone() {
    let control = format!("<w:tr><w:sdt><w:sdtContent>{}</w:sdtContent></w:sdt></w:tr>", cell(&["x"]));

    let body = fill(&grid_table(&[control]), json!({}));

    assert_eq!(text(&body), "x");
    assert!(body.contains("<w:tr>"), "{body}");
}

#[test]
fn leaves_a_part_a_raw_value_unbalanced_as_filled() {
    let body = fill(&format!("{}{}", paragraph(&["{@x}"]), only_row_loop()), json!({ "x": "<w:p><w:r>" }));

    assert!(body.contains("<w:p><w:r>"), "{body}");
}

fn part_with_loop(root: &str) -> String {
    format!("<w:{root} {WORD_NAMESPACE}>{}{}{}</w:{root}>", paragraph(&["{#l}"]), paragraph(&["x"]), paragraph(&["{/l}"]))
}

#[test]
fn gives_a_header_and_a_footer_a_paragraph_when_a_loop_empties_them() {
    for root in ["hdr", "ftr"] {
        let filled = fill_part(&part_with_loop(root), &json!({ "l": [] }), &FillOptions::core()).unwrap();

        assert!(filled.ends_with(&format!("<w:p/></w:{root}>")), "{filled}");
    }
}

#[test]
fn gives_a_text_box_a_paragraph_when_a_loop_empties_it() {
    let loop_paragraphs = format!("{}{}{}", paragraph(&["{#l}"]), paragraph(&["x"]), paragraph(&["{/l}"]));
    let shape = format!("<w:p><w:r><w:pict><w:txbxContent>{loop_paragraphs}</w:txbxContent></w:pict></w:r></w:p>");

    let body = fill(&shape, json!({ "l": [] }));

    assert!(body.contains("<w:txbxContent><w:p/></w:txbxContent>"), "{body}");
}

fn package(body: &str) -> Vec<(String, Vec<u8>)> {
    vec![
        ("[Content_Types].xml".to_string(), br#"<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"></Types>"#.to_vec()),
        ("word/_rels/document.xml.rels".to_string(), br#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"></Relationships>"#.to_vec()),
        ("word/document.xml".to_string(), document(body).into_bytes()),
    ]
}

fn filled_body(template: &PackageTemplate, data: Value) -> String {
    let parts = template.render(&data);
    let (_, bytes): &(String, Cow<[u8]>) = parts.iter().find(|(name, _)| name == "word/document.xml").unwrap();
    String::from_utf8_lossy(bytes).into_owned()
}

#[test]
fn repairs_a_package_after_its_subtemplates_print() {
    let template = PackageTemplate::parse(package(&in_cell(&paragraph(&["{:include d}"]))), &FillOptions::default()).unwrap();

    assert_cell_ends_in_paragraph(&filled_body(&template, json!({})), "no include");
}

#[test]
fn does_not_add_a_paragraph_after_a_subtemplate_that_ends_in_one() {
    let mut template = PackageTemplate::parse(package(&in_cell(&paragraph(&["{:include d}"]))), &FillOptions::default()).unwrap();
    let include = Subtemplate::parse(package(&paragraph(&["sub"])), &FillOptions::default(), docx_fill::IncludeMode::Templated, docx_fill::IncludeStyles::UseMain).unwrap();
    template.add_include("sub", include);

    let body = filled_body(&template, json!({ "d": "sub" }));

    assert_eq!(text(&body), "subafter");
    assert!(!body.contains("<w:p/>"), "{body}");
}

#[test]
fn drops_an_empty_last_page_left_by_a_table_it_removed() {
    let page_break = r#"<w:p><w:r><w:br w:type="page"/></w:r></w:p>"#;
    let body = format!("{}{page_break}{}{SECTION}", paragraph(&["a"]), only_row_loop());
    let template = PackageTemplate::parse(package(&body), &FillOptions::default()).unwrap();

    let filled = filled_body(&template, json!({ "l": [] }));

    assert!(!filled.contains("w:type=\"page\""), "{filled}");
    assert!(!filled.contains("<w:tbl"), "{filled}");
}
