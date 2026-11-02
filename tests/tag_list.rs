//! `Template::tags` and `PackageTemplate::tags` list a template's tags in the shape of its data.

mod common;

use common::{cell, document, paragraph, row, table};
use docx_fill::{FillOptions, PackageTemplate, Template, TemplateTag};

fn tags(body: &str) -> Vec<TemplateTag> {
    let options = FillOptions { module_prefixes: vec!["$".into()], ..FillOptions::default() };
    Template::parse(&document(body), &options).unwrap().tags()
}

/// Each tag as `(kind, name)`, a loop's tags after it and indented.
fn outline(tags: &[TemplateTag]) -> Vec<String> {
    let mut lines = Vec::new();
    for tag in tags {
        lines.push(format!("{:?} {}", tag.kind, tag.name));
        lines.extend(outline(&tag.children).into_iter().map(|line| format!("  {line}")));
    }
    lines
}

#[test]
fn nests_tags_inside_their_loops() {
    let body = paragraph(&["{title}"]) + &paragraph(&["{#users}{name}{#pets}{kind}{/pets}{^pets}none{/}{/users}"]) + &paragraph(&["{footer}"]);

    assert_eq!(outline(&tags(&body)), [
        "Value title",
        "Loop users",
        "  Value name",
        "  Loop pets",
        "    Value kind",
        "  InvertedLoop pets",
        "Value footer",
    ]);
}

#[test]
fn nests_tags_inside_row_loops_and_paragraph_loops() {
    let body = table(&[row(&[cell(&["{-w:tr rows}{a}"]), cell(&["{b}{/rows}"])])]) + &paragraph(&["{#items}"]) + &paragraph(&["{.}"]) + &paragraph(&["{/items}"]);

    assert_eq!(outline(&tags(&body)), ["Loop rows", "  Value a", "  Value b", "Loop items", "  Value ."]);
}

#[test]
fn keeps_the_tag_as_written() {
    let listed = tags(&paragraph(&["{#users}{ name }{/users}"]));

    assert_eq!(listed[0].written, "#users");
    assert_eq!(listed[0].children[0].written, " name ");
}

#[test]
fn names_every_tag_kind() {
    let paragraphs = [
        "{value}", "{@raw}", "{$module}", "{~~html}", "{x}{~inline}", "{%image}", "{%%centered}", "{:stylepar style}",
        "{:stylestart range}x{:/style}", "{::stylerun global}", "{r@run}", "{?placeholder}", "x{:footnote note}", "{:table data}",
        "{:#grid cells}", "{text}", "{:/grid}", "{:include sub}", "{:segment part}", "{s}", "{:segment/}", "{:includesegment part}", "{:page props}",
    ];
    let body: String = paragraphs.iter().map(|text| paragraph(&[text])).collect();
    let merge = table(&[row(&[cell(&["{:stylecell color}{:merge-cells-col 1}"])])]);

    assert_eq!(outline(&tags(&(body + &merge))), [
        "Value value", "Raw raw", "Module module", "RichText html", "Value x", "RichText inline", "Image image", "Image centered",
        "Style style", "Style range", "Style global", "RawRun run", "Placeholder placeholder", "Footnote note", "Table data",
        "Grid cells", "  Value text", "Include sub", "Segment part", "  Value s", "IncludeSegment part", "Section props", "Style color",
        "TableMerge ",
    ]);
}

#[test]
fn lists_each_template_parts_tags() {
    let parts = [
        ("word/document.xml", document(&paragraph(&["{body}"]))),
        ("word/header1.xml", r#"<w:hdr xmlns:w="x"><w:p><w:r><w:t>{head}</w:t></w:r></w:p></w:hdr>"#.to_string()),
        ("word/media/image1.png", "png".to_string()),
    ];
    let template = PackageTemplate::parse(parts.into_iter().map(|(name, xml)| (name.to_string(), xml.into_bytes())), &FillOptions::default()).unwrap();
    let listed: Vec<(&str, Vec<String>)> = template.tags().into_iter().map(|(part, tags)| (part, outline(&tags))).collect();

    assert_eq!(listed, [("word/document.xml", vec!["Value body".to_string()]), ("word/header1.xml", vec!["Value head".to_string()])]);
}

