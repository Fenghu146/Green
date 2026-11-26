use std::path::Path;

use lopdf::content::{Content, Operation};
use lopdf::{Dictionary, Document, Object, Stream, dictionary};

use crate::audit::{self, Audit, Font, Warning};
use crate::doc::{Align, Block, Inline, Item, Style};
use crate::typst::{self, PREAMBLE};

fn body(blocks: &[Block]) -> String {
    let out = typst::source(blocks).expect("the source");
    out.strip_prefix(PREAMBLE)
        .expect("the preamble first")
        .to_string()
}

fn paragraph(text: &str) -> Block {
    Block::Paragraph(Inline::plain(text))
}

#[test]
fn mark_prints_as_highlight() {
    let inline = Inline::styled("前 高亮 后", vec![(4..10, Style::Mark)]);
    assert_eq!(
        body(&[Block::Paragraph(inline)]),
        "前 #highlight[高亮] 后\n\n"
    );
}

#[test]
fn nested_styles_nest() {
    let inline = Inline::styled("ab", vec![(0..2, Style::Mark), (0..1, Style::Strong)]);
    assert_eq!(body(&[Block::Paragraph(inline)]), "#highlight[*a*b]\n\n");
}

#[test]
fn link_prints_as_a_link() {
    let inline = Inline {
        text: "看这里".into(),
        styles: vec![(0..9, Style::Link)],
        links: vec![(0..9, "https://example.com".into())],
    };
    assert_eq!(
        body(&[Block::Paragraph(inline)]),
        "#link(\"https://example.com\")[看这里]\n\n"
    );
}

#[test]
fn inline_math_goes_through_green_math() {
    let inline = Inline::styled("a_n", vec![(0..3, Style::Math)]);
    assert_eq!(body(&[Block::Paragraph(inline)]), "$a_n$\n\n");
}

#[test]
fn a_formula_outside_the_whitelist_is_named() {
    let error = typst::source(&[Block::Math("\\notacommand x".into())]).expect_err("a refusal");
    let said = format!("{error:#}");
    assert!(said.contains("notacommand"), "{said}");
}

#[test]
fn text_punctuation_is_literal() {
    let text = "# $ * _ ` [ ] < > @ ~ \\ - + = / ' \"";
    let escaped = "\\# \\$ \\* \\_ \\` \\[ \\] \\< \\> \\@ \\~ \\\\ \\- \\+ \\= \\/ \\' \\\"";
    assert_eq!(body(&[paragraph(text)]), format!("{escaped}\n\n"));
}

#[test]
fn a_paragraph_that_starts_with_a_number_keeps_it() {
    assert_eq!(body(&[paragraph("2.5 是")]), "2\\.5 是\n\n");
    assert_eq!(body(&[paragraph("3) 是")]), "3\\) 是\n\n");
}

#[test]
fn marks_the_bundled_fonts_lack_become_symbols() {
    assert_eq!(
        body(&[paragraph("✅ 好了 ❌ 不行")]),
        "#sym.checkmark 好了 #sym.crossmark 不行\n\n"
    );
}

#[test]
fn headings_count_their_equals() {
    assert_eq!(
        body(&[Block::Heading(3, Inline::plain("标题"))]),
        "=== 标题\n\n"
    );
}

#[test]
fn ordered_lists_start_at_their_number() {
    let items = vec![
        Item {
            task: None,
            blocks: vec![paragraph("甲")],
        },
        Item {
            task: None,
            blocks: vec![paragraph("乙")],
        },
    ];
    assert_eq!(
        body(&[Block::List {
            start: Some(5),
            items
        }]),
        "5. 甲\n6. 乙\n\n"
    );
}

#[test]
fn a_nested_list_indents_under_its_item() {
    let items = vec![Item {
        task: None,
        blocks: vec![
            paragraph("外"),
            Block::List {
                start: None,
                items: vec![Item {
                    task: None,
                    blocks: vec![paragraph("内")],
                }],
            },
        ],
    }];
    assert_eq!(
        body(&[Block::List { start: None, items }]),
        "- 外\n\n  - 内\n\n"
    );
}

#[test]
fn task_items_show_their_box() {
    let items = vec![
        Item {
            task: Some(false),
            blocks: vec![paragraph("没做")],
        },
        Item {
            task: Some(true),
            blocks: vec![paragraph("做了")],
        },
    ];
    assert_eq!(
        body(&[Block::List { start: None, items }]),
        "- #sym.square.stroked 没做\n- #sym.checkmark 做了\n\n"
    );
}

#[test]
fn a_table_takes_its_alignment_and_cells() {
    let table = Block::Table {
        aligns: vec![Align::Left, Align::Right],
        head: vec![vec![paragraph("名")], vec![paragraph("值")]],
        rows: vec![vec![vec![paragraph("甲")], vec![paragraph("1")]]],
    };
    assert_eq!(
        body(&[table]),
        "#table(\n  columns: 2,\n  align: (left, right),\n  table.header([名], [值], ),\n  [甲],\n  [1],\n)\n\n"
    );
}

#[test]
fn a_code_fence_grows_past_its_content() {
    let code = Block::Code {
        language: Some("rust".into()),
        text: "```\ninner\n```".into(),
    };
    assert_eq!(body(&[code]), "``````rust\n```\ninner\n```\n``````\n\n");
}

#[test]
fn footnotes_are_listed_at_the_end() {
    let blocks = vec![
        paragraph("正文"),
        Block::Footnote {
            label: "1".into(),
            body: vec![paragraph("注")],
        },
    ];
    assert_eq!(
        body(&blocks),
        "正文\n\n#v(1.5em)\n#line(length: 100%, stroke: 0.4pt)\n#text(size: 0.85em)[\n[1] 注\n\n]\n"
    );
}

#[test]
fn thresholds_call_a_heavy_export_heavy() {
    let audit = Audit {
        pages: 1,
        bytes: 4 * 1024 * 1024,
        fonts: vec![Font {
            name: "Heavy".into(),
            bytes: 4 * 1024 * 1024,
        }],
        image_refs: 1,
        images: 1,
        image_bytes: 3 * 1024 * 1024,
        widest_page: Some(1),
        widest_coverage: 0.5,
        ..Audit::default()
    };
    assert_eq!(
        audit.warnings(),
        vec![
            Warning::MeanPage {
                bytes: 4 * 1024 * 1024
            },
            Warning::Font {
                name: "Heavy".into(),
                bytes: 4 * 1024 * 1024
            },
            Warning::Images { share: 0.75 },
            Warning::Coverage {
                page: 1,
                share: 0.5
            },
        ]
    );
}

#[test]
fn a_light_export_has_no_warnings() {
    let audit = Audit {
        pages: 2,
        bytes: 40_000,
        fonts: vec![Font {
            name: "NotoSansCJKsc-Regular".into(),
            bytes: 20_000,
        }],
        compressed: true,
        ..Audit::default()
    };
    assert!(audit.warnings().is_empty());
    assert_eq!(
        audit.line(),
        "export pdf: pages=2 bytes=40000 fonts=[{\"name\":\"NotoSansCJKsc-Regular\",\"bytes\":20000}] images=0/0 (dup:0) compressed=true max_image_coverage=0.0%"
    );
}

/// A one-page 400×400 PDF: a font of `font_bytes`, and each picture's bytes
/// and placement matrix.
fn built(path: &Path, font_bytes: usize, images: &[(Vec<u8>, [i64; 6])]) {
    let mut doc = Document::with_version("1.7");
    let pages_id = doc.new_object_id();
    let page_id = doc.new_object_id();
    let mut xobjects = Dictionary::new();
    let mut operations = Vec::new();
    for (index, (content, matrix)) in images.iter().enumerate() {
        let id = doc.add_object(Stream::new(
            dictionary! {
                "Type" => "XObject",
                "Subtype" => "Image",
                "Width" => 4,
                "Height" => 4,
                "ColorSpace" => "DeviceGray",
                "BitsPerComponent" => 8,
            },
            content.clone(),
        ));
        let name = format!("Im{index}");
        xobjects.set(name.clone().into_bytes(), id);
        operations.push(Operation::new("q", vec![]));
        operations.push(Operation::new(
            "cm",
            matrix.iter().map(|value| Object::Integer(*value)).collect(),
        ));
        operations.push(Operation::new("Do", vec![Object::Name(name.into_bytes())]));
        operations.push(Operation::new("Q", vec![]));
    }
    let mut contents = Stream::new(
        dictionary! {},
        Content { operations }.encode().expect("the content"),
    );
    let mut content = contents.content.clone();
    content.extend(std::iter::repeat_n(b' ', 4096));
    contents.set_content(content);
    contents.compress().expect("a flate stream");
    let contents_id = doc.add_object(contents);

    let file_id = doc.add_object(Stream::new(
        dictionary! { "Length1" => font_bytes as i64 },
        vec![0u8; font_bytes],
    ));
    let descriptor_id = doc.add_object(dictionary! {
        "Type" => "FontDescriptor",
        "FontName" => "Sub",
        "Flags" => 4,
        "FontBBox" => vec![0.into(), 0.into(), 0.into(), 0.into()],
        "ItalicAngle" => 0,
        "Ascent" => 0,
        "Descent" => 0,
        "CapHeight" => 0,
        "StemV" => 0,
        "FontFile2" => file_id,
    });
    let font_id = doc.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "TrueType",
        "BaseFont" => "ABCDEF+TestSans-Identity-H",
        "FontDescriptor" => descriptor_id,
    });

    doc.objects.insert(
        page_id,
        Object::Dictionary(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "MediaBox" => vec![0.into(), 0.into(), 400.into(), 400.into()],
            "Resources" => dictionary! {
                "XObject" => xobjects,
                "Font" => dictionary! { "F1" => font_id },
            },
            "Contents" => contents_id,
        }),
    );
    doc.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! {
            "Type" => "Pages",
            "Kids" => vec![page_id.into()],
            "Count" => 1,
        }),
    );
    let catalog = doc.add_object(dictionary! {
        "Type" => "Catalog",
        "Pages" => pages_id,
    });
    doc.trailer.set("Root", catalog);
    doc.save(path).expect("the file");
}

fn scratch(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("green-export-{}-{name}.pdf", std::process::id()))
}

#[test]
fn an_audit_reads_the_fonts_pictures_and_coverage_back() {
    let path = scratch("audit");
    let same = vec![9u8; 48];
    built(
        &path,
        1200,
        &[
            (same.clone(), [200, 0, 0, 200, 0, 0]),
            (same, [100, 0, 0, 100, 0, 0]),
        ],
    );
    let audit = audit::read(&path).expect("the audit");
    let _ = std::fs::remove_file(&path);
    assert_eq!(audit.pages, 1);
    assert!(audit.bytes > 0);
    assert_eq!(
        audit.fonts,
        vec![Font {
            name: "TestSans".into(),
            bytes: 1200
        }]
    );
    assert_eq!(audit.image_refs, 2);
    assert_eq!(audit.images, 1);
    assert_eq!(audit.image_bytes, 96);
    assert!(audit.image_filters.is_empty());
    assert!(audit.compressed);
    assert_eq!(audit.widest_page, Some(1));
    assert!((audit.widest_coverage - 0.25).abs() < 1e-6, "{audit:?}");
    assert_eq!(
        audit.line(),
        format!(
            "export pdf: pages=1 bytes={} fonts=[{{\"name\":\"TestSans\",\"bytes\":1200}}] images=2/1 (dup:1, filters=[]) compressed=true max_image_coverage=25.0%",
            audit.bytes
        )
    );
}
