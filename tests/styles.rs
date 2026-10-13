//! Styles (cycle 018): per-column bold/fill through the contract,
//! asserted on the emitted bytes and round-tripped by value.

#[allow(dead_code)]
mod common;

use std::path::PathBuf;

use thoth::contract::{Colour, Column, DynamicSheet, Kind, StaticColumn, StaticSheet, Style};
use thoth::xlsx::ColumnData;

fn temp_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("thoth-styles-{}-{name}.xlsx", std::process::id()))
}

#[test]
fn style_index_is_arithmetic() {
    assert_eq!(Style::default().index(), None);
    assert_eq!(
        Style {
            bold: true,
            colour: None
        }
        .index(),
        Some(1)
    );
    let palette: [Colour; 6] = [
        Colour::Red,
        Colour::Green,
        Colour::Amber,
        Colour::Blue,
        Colour::Purple,
        Colour::Grey,
    ];
    for (i, colour) in palette.iter().enumerate() {
        assert_eq!(
            Style {
                bold: false,
                colour: Some(*colour)
            }
            .index(),
            Some(2 + 2 * i),
            "{colour:?} plain"
        );
        assert_eq!(
            Style {
                bold: true,
                colour: Some(*colour)
            }
            .index(),
            Some(3 + 2 * i),
            "{colour:?} bold"
        );
    }
}

#[test]
fn styled_values_round_trip_unchanged() {
    let example = common::styled();
    let cols = example.writer_columns();
    let template = common::Example::dynamic_sheet("data", &cols);
    let path: PathBuf = temp_path("roundtrip");
    let _ = std::fs::remove_file(&path);
    thoth::xlsx::write_workbook_file(&path, &[template]).unwrap();
    assert_eq!(common::read_sheet_cells(&path), example.expected);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn styled_bytes_carry_s_indices_and_styles_part() {
    let example = common::styled();
    let cols = example.writer_columns();
    let template = common::Example::dynamic_sheet("data", &cols);
    let path: PathBuf = temp_path("bytes");
    let _ = std::fs::remove_file(&path);
    thoth::xlsx::write_workbook_file(&path, &[template]).unwrap();
    let sheet: String =
        String::from_utf8(common::read_part(&path, "xl/worksheets/sheet1.xml")).unwrap();
    let styles: String = String::from_utf8(common::read_part(&path, "xl/styles.xml")).unwrap();
    let _ = std::fs::remove_file(&path);
    // Headers are byte-identical to the plain spelling: bold only.
    for header in [
        "<c r=\"A1\" t=\"inlineStr\" s=\"1\"><is><t>t</t></is></c>",
        "<c r=\"B1\" t=\"inlineStr\" s=\"1\"><is><t>px</t></is></c>",
        "<c r=\"C1\" t=\"inlineStr\" s=\"1\"><is><t>name</t></is></c>",
        "<c r=\"D1\" t=\"inlineStr\" s=\"1\"><is><t>ok</t></is></c>",
    ] {
        assert!(sheet.contains(header), "header {header} in {sheet}");
    }
    // Valued data cells: A bold=1, B red=2, C green+bold=5, D amber=6.
    for cell in [
        "<c r=\"A2\" s=\"1\"><v>100</v></c>",
        "<c r=\"B2\" s=\"2\"><v>10.5</v></c>",
        "<c r=\"C2\" s=\"5\" t=\"inlineStr\">",
        "<c r=\"D2\" s=\"6\" t=\"b\"><v>1</v></c>",
        "<c r=\"A4\" s=\"1\"><v>300</v></c>",
        "<c r=\"B4\" s=\"2\"><v>12.25</v></c>",
        "<c r=\"D4\" s=\"6\" t=\"b\"><v>0</v></c>",
    ] {
        assert!(sheet.contains(cell), "cell {cell} in {sheet}");
    }
    // Blank cells in styled columns carry the index exactly.
    for cell in [
        "<c r=\"A3\" s=\"1\"/>",
        "<c r=\"B3\" s=\"2\"/>",
        "<c r=\"C3\" s=\"5\"/>",
        "<c r=\"D3\" s=\"6\"/>",
    ] {
        assert!(sheet.contains(cell), "blank {cell} in {sheet}");
    }
    // Palette fills in palette order and the cellXfs table.
    assert!(styles.contains("<fonts count=\"2\">"), "{styles}");
    assert!(styles.contains("<fills count=\"8\">"), "{styles}");
    assert!(styles.contains("<cellXfs count=\"14\">"), "{styles}");
    let mut at: usize = 0;
    for rgb in [
        "FFFF0000", "FF00B050", "FFFFC000", "FF0070C0", "FF7030A0", "FFBFBFBF",
    ] {
        let needle: String = format!("<fgColor rgb=\"{rgb}\"/>");
        let found: usize = styles[at..]
            .find(&needle)
            .map(|i| at + i)
            .unwrap_or_else(|| panic!("fill {needle} in {styles}"));
        at = found + needle.len();
    }
    // The full cellXfs table, in index order: plain, bold, then a
    // plain/bold pair per palette colour (fillIds 2..=7).
    let mut xf_at: usize = styles.find("<cellXfs count=\"14\">").unwrap();
    for xf in [
        "<xf numFmtId=\"0\" fontId=\"0\" fillId=\"0\" borderId=\"0\" xfId=\"0\"/>",
        "<xf numFmtId=\"0\" fontId=\"1\" fillId=\"0\" borderId=\"0\" xfId=\"0\" applyFont=\"1\"/>",
        "<xf numFmtId=\"0\" fontId=\"0\" fillId=\"2\" borderId=\"0\" xfId=\"0\" applyFill=\"1\"/>",
        "<xf numFmtId=\"0\" fontId=\"1\" fillId=\"2\" borderId=\"0\" xfId=\"0\" applyFont=\"1\" applyFill=\"1\"/>",
        "<xf numFmtId=\"0\" fontId=\"0\" fillId=\"3\" borderId=\"0\" xfId=\"0\" applyFill=\"1\"/>",
        "<xf numFmtId=\"0\" fontId=\"1\" fillId=\"3\" borderId=\"0\" xfId=\"0\" applyFont=\"1\" applyFill=\"1\"/>",
        "<xf numFmtId=\"0\" fontId=\"0\" fillId=\"4\" borderId=\"0\" xfId=\"0\" applyFill=\"1\"/>",
        "<xf numFmtId=\"0\" fontId=\"1\" fillId=\"4\" borderId=\"0\" xfId=\"0\" applyFont=\"1\" applyFill=\"1\"/>",
        "<xf numFmtId=\"0\" fontId=\"0\" fillId=\"5\" borderId=\"0\" xfId=\"0\" applyFill=\"1\"/>",
        "<xf numFmtId=\"0\" fontId=\"1\" fillId=\"5\" borderId=\"0\" xfId=\"0\" applyFont=\"1\" applyFill=\"1\"/>",
        "<xf numFmtId=\"0\" fontId=\"0\" fillId=\"6\" borderId=\"0\" xfId=\"0\" applyFill=\"1\"/>",
        "<xf numFmtId=\"0\" fontId=\"1\" fillId=\"6\" borderId=\"0\" xfId=\"0\" applyFont=\"1\" applyFill=\"1\"/>",
        "<xf numFmtId=\"0\" fontId=\"0\" fillId=\"7\" borderId=\"0\" xfId=\"0\" applyFill=\"1\"/>",
        "<xf numFmtId=\"0\" fontId=\"1\" fillId=\"7\" borderId=\"0\" xfId=\"0\" applyFont=\"1\" applyFill=\"1\"/>",
    ] {
        let found: usize = styles[xf_at..]
            .find(xf)
            .map(|i| xf_at + i)
            .unwrap_or_else(|| panic!("xf {xf} in {styles}"));
        xf_at = found + xf.len();
    }
    // The arena bytes embed exactly the oracle sheet.
    let oracle: Vec<u8> =
        thoth::xlsx::sheet_xml_template(&common::Example::dynamic_sheet("data", &cols)).unwrap();
    assert!(
        sheet.as_bytes() == oracle.as_slice(),
        "stored sheet differs from oracle"
    );
}

#[test]
fn macro_styles_match_handwritten_bytes() {
    let t: Vec<i64> = vec![1, 2, 3];
    let names: Vec<Option<&str>> = vec![Some("x"), None, Some(" y")];
    let macro_bytes = thoth::xlsx::build_workbook(&[thoth::dynamic_sheet!("data",
        i64 [bold] "t" => &t,
        str [bold, green] "name" => &names,
    )])
    .unwrap();
    let cols: Vec<(Column, ColumnData<'_>)> = vec![
        (
            Column {
                name: "t".to_string(),
                kind: Kind::I64,
                style: Style {
                    bold: true,
                    colour: None,
                },
            },
            ColumnData::I64(&t),
        ),
        (
            Column {
                name: "name".to_string(),
                kind: Kind::Str,
                style: Style {
                    bold: true,
                    colour: Some(Colour::Green),
                },
            },
            ColumnData::Str(&names),
        ),
    ];
    let hand = DynamicSheet::known("data".to_string(), &cols);
    assert_eq!(macro_bytes, thoth::xlsx::build_workbook(&[hand]).unwrap());
    // Colour and bold in the other order spell the same style.
    let swapped = thoth::xlsx::build_workbook(&[thoth::dynamic_sheet!("data",
        i64 [bold] "t" => &t,
        str [green, bold] "name" => &names,
    )])
    .unwrap();
    assert_eq!(macro_bytes, swapped);
    // `columns!` carries the same styles.
    let bound = thoth::columns!(i64 [bold] "t" => &t, str [green, bold] "name" => &names);
    let hand2 = DynamicSheet::known("data".to_string(), &bound);
    assert_eq!(macro_bytes, thoth::xlsx::build_workbook(&[hand2]).unwrap());
    // Static macro, colour only and bold-colour.
    let fixed_macro = thoth::static_sheet!("fixed",
        i64 [red] "t" => [1, 2],
        str [bold, blue] "name" => [Some("x"), None],
    );
    let fixed_hand: StaticSheet<2, 2> = StaticSheet::new(
        "fixed",
        [
            StaticColumn::I64 {
                name: "t",
                style: Style {
                    bold: false,
                    colour: Some(Colour::Red),
                },
                data: [1, 2],
            },
            StaticColumn::Str {
                name: "name",
                style: Style {
                    bold: true,
                    colour: Some(Colour::Blue),
                },
                data: [Some("x"), None],
            },
        ],
    );
    assert_eq!(
        thoth::xlsx::build_workbook(&[fixed_macro]).unwrap(),
        thoth::xlsx::build_workbook(&[fixed_hand]).unwrap()
    );
}
