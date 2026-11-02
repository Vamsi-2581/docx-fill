//! Mirrors `apps/web/src/components/pdf/wordcraft/__tests__/verticalTables.test.ts`, with the
//! `{-w:tc …}` cell loops `rewriteVerticalTables` writes.

mod common;

use common::{cell, fill, read_rows, row, table};
use serde_json::json;

#[test]
fn repeats_the_tagged_cell_once_per_item() {
    let xml = fill(
        &table(&[row(&[cell(&["Label"]), cell(&["{-w:tc grid:a.value}{name}{/grid:a.value}"])])]),
        json!({ "grid:a.value": [{ "name": "one" }, { "name": "two" }, { "name": "three" }] }),
    );

    assert_eq!(read_rows(&xml), [["Label", "one", "two", "three"]]);
}

#[test]
fn repeats_every_cell_from_the_opening_tags_cell_to_the_closing_tags() {
    let xml = fill(
        &table(&[row(&[cell(&["{-w:tc cols}{a}"]), cell(&["{b}{/cols}"]), cell(&["End"])])]),
        json!({ "cols": [{ "a": "a1", "b": "b1" }, { "a": "a2", "b": "b2" }] }),
    );

    assert_eq!(read_rows(&xml), [["a1", "b1", "a2", "b2", "End"]]);
}

#[test]
fn repeats_columns_in_each_row_of_a_row_loop_with_that_rows_items() {
    let xml = fill(
        &table(&[
            row(&[cell(&["Name"]), cell(&["{-w:tc vt}{header}{/vt}"])]),
            row(&[cell(&["{-w:tr rows}{label}"]), cell(&["{-w:tc vt}{value}{/vt}{/rows}"])]),
        ]),
        json!({
            "vt": [{ "header": "H1" }, { "header": "H2" }],
            "rows": [
                { "label": "r1", "vt": [{ "value": "1a" }, { "value": "1b" }] },
                { "label": "r2", "vt": [{ "value": "2a" }, { "value": "2b" }] }
            ]
        }),
    );

    assert_eq!(read_rows(&xml), [["Name", "H1", "H2"], ["r1", "1a", "1b"], ["r2", "2a", "2b"]]);
}

#[test]
fn reads_a_tag_word_split_across_runs() {
    let xml = fill(&table(&[row(&[cell(&["{-w:t", "c items}{x}{/it", "ems}"])])]), json!({ "items": [{ "x": "1" }, { "x": "2" }] }));

    assert_eq!(read_rows(&xml), [["1", "2"]]);
}

#[test]
fn leaves_the_row_without_the_cells_when_the_list_is_empty() {
    let xml = fill(&table(&[row(&[cell(&["Label"]), cell(&["{-w:tc items}{x}{/items}"])])]), json!({ "items": [] }));

    assert_eq!(read_rows(&xml), [["Label"]]);
}

#[test]
fn repeats_cells_between_vertical_table_tags() {
    let xml = fill(
        &table(&[row(&[cell(&["Name"]), cell(&["{:vt#vt}{header}{:vt/vt}"])]), row(&[cell(&["{-w:tr rows}{label}"]), cell(&["{:vt# vt }{value}{:v", "t/vt}{/rows}"])])]),
        json!({
            "vt": [{ "header": "H1" }, { "header": "H2" }],
            "rows": [{ "label": "r1", "vt": [{ "value": "1a" }, { "value": "1b" }] }]
        }),
    );

    assert_eq!(read_rows(&xml), [["Name", "H1", "H2"], ["r1", "1a", "1b"]]);
}

fn sized_cell(width: u32, text: &str) -> String {
    format!(r#"<w:tc><w:tcPr><w:tcW w:w="{width}" w:type="dxa"/></w:tcPr><w:p><w:r><w:t>{text}</w:t></w:r></w:p></w:tc>"#)
}

/// A table with a grid of `widths` and one row of cells of those widths.
fn gridded(widths: &[u32], texts: &[&str]) -> String {
    let grid: String = widths.iter().map(|width| format!(r#"<w:gridCol w:w="{width}"/>"#)).collect();
    let cells: String = widths.iter().zip(texts).map(|(width, text)| sized_cell(*width, text)).collect();
    format!("<w:tbl><w:tblGrid>{grid}</w:tblGrid><w:tr>{cells}</w:tr></w:tbl>")
}

fn widths(xml: &str, element: &str) -> Vec<u32> {
    xml.split(&format!("<{element} w:w=\"")).skip(1).map(|rest| rest[..rest.find('"').unwrap()].parse().unwrap()).collect()
}

#[test]
fn widens_the_grid_for_repeated_columns_keeping_the_tables_width() {
    let xml = fill(&gridded(&[2000, 4000], &["{:vt#c}a", "b{:vt/c}"]), json!({ "c": [{}, {}, {}] }));

    assert_eq!(widths(&xml, "w:gridCol"), [666, 1333, 666, 1333, 666, 1333]);
    assert_eq!(widths(&xml, "w:tcW"), [666, 1333, 666, 1333, 666, 1333]);
}

#[test]
fn narrows_the_grid_for_removed_columns_keeping_the_tables_width() {
    let xml = fill(&gridded(&[3000, 3000], &["Label", "{:vt#c}x{:vt/c}"]), json!({ "c": [] }));

    assert_eq!(widths(&xml, "w:gridCol"), [6000]);
    assert_eq!(widths(&xml, "w:tcW"), [6000]);
}

#[test]
fn leaves_a_grid_that_still_matches() {
    let template = gridded(&[3000, 3000], &["Label", "{:vt#c}x{:vt/c}"]);
    let xml = fill(&template, json!({ "c": [{}] }));

    assert_eq!(widths(&xml, "w:gridCol"), [3000, 3000]);
}

#[test]
fn refits_a_nested_table_apart_from_the_table_around_it() {
    let inner = gridded(&[1000], &["{-w:tc c}x{/c}"]);
    let outer = format!(r#"<w:tbl><w:tblGrid><w:gridCol w:w="5000"/></w:tblGrid><w:tr><w:tc><w:tcPr><w:tcW w:w="5000" w:type="dxa"/></w:tcPr>{inner}<w:p/></w:tc></w:tr></w:tbl>"#);
    let xml = fill(&outer, json!({ "c": [{}, {}] }));

    assert_eq!(widths(&xml, "w:gridCol"), [5000, 500, 500]);
}
