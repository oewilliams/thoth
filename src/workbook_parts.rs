//! Named sheets: name validation and the workbook parts that carry
//! the names. Split from `xlsx` (STYLE rule 1 file tripwire): the
//! fragment builders, their exact-capacity proofs, and the name rules
//! are one concern; sheet emission and archive assembly stay there.

use crate::emit::{escape_into, push_usize};
use crate::static_parts::{
    CONTENT_TYPES_HEAD, CONTENT_TYPES_SHEET_CLOSE, CONTENT_TYPES_SHEET_OPEN, CONTENT_TYPES_TAIL,
    WORKBOOK_HEAD, WORKBOOK_RELS_HEAD, WORKBOOK_RELS_SHEET_CLOSE, WORKBOOK_RELS_SHEET_OPEN,
    WORKBOOK_RELS_SHEET_TARGET, WORKBOOK_RELS_STYLES_CLOSE, WORKBOOK_RELS_STYLES_OPEN,
    WORKBOOK_RELS_TAIL, WORKBOOK_SHEET_CLOSE, WORKBOOK_SHEET_ID, WORKBOOK_SHEET_OPEN,
    WORKBOOK_SHEET_RID, WORKBOOK_TAIL,
};
use crate::xlsx::XlsxError;

/// Decimal digit width of `n` (1 for 0..=9, 2 for 10..=99, ...).
/// The `ilog10` spelling (benched ~20% faster than the division loop
/// over 1..=1M, threshold ladder between the two).
pub(crate) fn digit_width(n: usize) -> usize {
    (n.max(1).ilog10() + 1) as usize
}

/// Sum of `digit_width(i)` for `i` in 1..=`n`, in closed form: whole
/// decades at one width each, plus the partial decade. Lets the cap
/// spellings below price every sheet index without a per-index loop.
pub(crate) fn digit_width_sum(n: usize) -> usize {
    let mut total: usize = 0;
    let mut decade: usize = 1;
    let mut width: usize = 1;
    loop {
        if n < decade * 10 {
            return total + (n + 1 - decade) * width;
        }
        total += 9 * decade * width;
        decade *= 10;
        width += 1;
    }
}

/// Byte length of `name` after XML escaping (see `escape_into`):
/// `&` -> `&amp;` (5), `<`/`>` -> 4, `"`/`'` -> 6, else UTF-8 length.
pub(crate) fn escaped_len(name: &str) -> usize {
    name.chars()
        .map(|c| match c {
            '&' => 5,
            '<' => 4,
            '>' => 4,
            '"' => 6,
            '\'' => 6,
            _ => c.len_utf8(),
        })
        .sum()
}

pub(crate) fn escape_sheet_name(name: &str) -> String {
    let mut escaped = String::with_capacity(escaped_len(name));
    escape_into(&mut escaped, name);
    escaped
}

/// Exact byte length of the assembled `[Content_Types].xml` for
/// `sheet_count` sheets: head + tail plus, per sheet, the fragment
/// pair and the 1-based index digits. The assembly allocates once
/// from this spelling, so no realloc occurs on that path (pinned by
/// `workbook_parts_cap_holds`, after `sheet_cap_holds`).
pub(crate) fn content_types_cap(sheet_count: usize) -> usize {
    CONTENT_TYPES_HEAD.len()
        + CONTENT_TYPES_TAIL.len()
        + sheet_count * (CONTENT_TYPES_SHEET_OPEN.len() + CONTENT_TYPES_SHEET_CLOSE.len())
        + digit_width_sum(sheet_count)
}

pub(crate) fn content_types_xml(sheet_count: usize) -> String {
    let cap: usize = content_types_cap(sheet_count);
    let mut out = String::with_capacity(cap);
    out.push_str(CONTENT_TYPES_HEAD);
    for index in 1..=sheet_count {
        out.push_str(CONTENT_TYPES_SHEET_OPEN);
        push_usize(&mut out, index);
        out.push_str(CONTENT_TYPES_SHEET_CLOSE);
    }
    out.push_str(CONTENT_TYPES_TAIL);
    debug_assert_eq!(out.len(), cap);
    out
}

/// Exact byte length of the assembled `xl/workbook.xml`: head + tail
/// plus, per sheet, the fragments, the escaped name, and the two
/// 1-based index digit spellings (`sheetId` and `r:id`).
pub(crate) fn workbook_cap(escaped: &[String]) -> usize {
    let names: usize = escaped.iter().map(|name| name.len()).sum();
    WORKBOOK_HEAD.len()
        + WORKBOOK_TAIL.len()
        + escaped.len()
            * (WORKBOOK_SHEET_OPEN.len()
                + WORKBOOK_SHEET_ID.len()
                + WORKBOOK_SHEET_RID.len()
                + WORKBOOK_SHEET_CLOSE.len())
        + names
        + 2 * digit_width_sum(escaped.len())
}

pub(crate) fn workbook_xml(escaped: &[String]) -> String {
    let cap: usize = workbook_cap(escaped);
    let mut out = String::with_capacity(cap);
    out.push_str(WORKBOOK_HEAD);
    for (i, name) in escaped.iter().enumerate() {
        let index: usize = i + 1;
        out.push_str(WORKBOOK_SHEET_OPEN);
        out.push_str(name);
        out.push_str(WORKBOOK_SHEET_ID);
        push_usize(&mut out, index);
        out.push_str(WORKBOOK_SHEET_RID);
        push_usize(&mut out, index);
        out.push_str(WORKBOOK_SHEET_CLOSE);
    }
    out.push_str(WORKBOOK_TAIL);
    debug_assert_eq!(out.len(), cap);
    out
}

/// Exact byte length of the assembled `xl/_rels/workbook.xml.rels`:
/// head + tail plus, per sheet, the fragments and two index digit
/// spellings, plus the styles fragment at `sheet_count + 1`.
pub(crate) fn workbook_rels_cap(sheet_count: usize) -> usize {
    WORKBOOK_RELS_HEAD.len()
        + WORKBOOK_RELS_TAIL.len()
        + sheet_count
            * (WORKBOOK_RELS_SHEET_OPEN.len()
                + WORKBOOK_RELS_SHEET_TARGET.len()
                + WORKBOOK_RELS_SHEET_CLOSE.len())
        + 2 * digit_width_sum(sheet_count)
        + WORKBOOK_RELS_STYLES_OPEN.len()
        + digit_width(sheet_count + 1)
        + WORKBOOK_RELS_STYLES_CLOSE.len()
}

pub(crate) fn workbook_rels_xml(sheet_count: usize) -> String {
    let cap: usize = workbook_rels_cap(sheet_count);
    let mut out = String::with_capacity(cap);
    out.push_str(WORKBOOK_RELS_HEAD);
    for index in 1..=sheet_count {
        out.push_str(WORKBOOK_RELS_SHEET_OPEN);
        push_usize(&mut out, index);
        out.push_str(WORKBOOK_RELS_SHEET_TARGET);
        push_usize(&mut out, index);
        out.push_str(WORKBOOK_RELS_SHEET_CLOSE);
    }
    out.push_str(WORKBOOK_RELS_STYLES_OPEN);
    push_usize(&mut out, sheet_count + 1);
    out.push_str(WORKBOOK_RELS_STYLES_CLOSE);
    out.push_str(WORKBOOK_RELS_TAIL);
    debug_assert_eq!(out.len(), cap);
    out
}

pub(crate) fn sheet_part_name(index: usize) -> String {
    let mut name =
        String::with_capacity("xl/worksheets/sheet".len() + digit_width(index) + ".xml".len());
    name.push_str("xl/worksheets/sheet");
    push_usize(&mut name, index);
    name.push_str(".xml");
    name
}

/// Sheet-name validation at the workbook boundary (STYLE rule 8:
/// refused loudly, never repaired or trimmed). Checked once in
/// `build_workbook` before any sheet is emitted.
pub(crate) fn validate_sheet_names(names: &[&str]) -> Result<(), XlsxError> {
    if names.is_empty() {
        return Err(XlsxError::NoSheets);
    }
    for name in names {
        if name.is_empty() {
            return Err(XlsxError::SheetNameEmpty {
                name: name.to_string(),
            });
        }
        if name.chars().count() > 31 {
            return Err(XlsxError::SheetNameTooLong {
                name: name.to_string(),
            });
        }
        if let Some(ch) = name
            .chars()
            .find(|c| matches!(c, '[' | ']' | ':' | '*' | '?' | '/' | '\\') || c.is_control())
        {
            return Err(XlsxError::SheetNameInvalidChar {
                name: name.to_string(),
                ch,
            });
        }
        if name.starts_with('\'') || name.ends_with('\'') {
            return Err(XlsxError::SheetNameApostrophe {
                name: name.to_string(),
            });
        }
        if name.eq_ignore_ascii_case("History") {
            return Err(XlsxError::SheetNameReserved {
                name: name.to_string(),
            });
        }
    }
    for (i, name) in names.iter().enumerate() {
        for prev in &names[..i] {
            if prev.to_lowercase() == name.to_lowercase() {
                return Err(XlsxError::DuplicateSheetName {
                    name: name.to_string(),
                });
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        content_types_cap, content_types_xml, digit_width, digit_width_sum, escape_sheet_name,
        workbook_cap, workbook_rels_cap, workbook_rels_xml, workbook_xml,
    };
    use crate::contract::DynamicSheet;
    use crate::contract::{Column, Kind, Style};
    use crate::static_parts::{
        CONTENT_TYPES_HEAD, CONTENT_TYPES_SHEET_CLOSE, CONTENT_TYPES_SHEET_OPEN,
        CONTENT_TYPES_TAIL, RELS, STYLES, WORKBOOK_HEAD, WORKBOOK_RELS_HEAD,
        WORKBOOK_RELS_SHEET_CLOSE, WORKBOOK_RELS_SHEET_OPEN, WORKBOOK_RELS_SHEET_TARGET,
        WORKBOOK_RELS_STYLES_CLOSE, WORKBOOK_RELS_STYLES_OPEN, WORKBOOK_RELS_TAIL,
        WORKBOOK_SHEET_CLOSE, WORKBOOK_SHEET_ID, WORKBOOK_SHEET_OPEN, WORKBOOK_SHEET_RID,
        WORKBOOK_TAIL,
    };
    use crate::xlsx::{ColumnData, build_workbook};

    /// Tripwire on the frozen statics: `RELS`/`STYLES` stay whole;
    /// the three sheet-naming parts are pinned as fragments plus a
    /// cat-identity assertion against the pre-011 whole-part pins.
    /// `crc32` is a change detector, round-trip is the oracle.
    #[test]
    fn static_parts_are_pinned() {
        let whole: [(&str, &str, usize, u32); 2] = [
            ("_rels/.rels", RELS, 296, 0x82C7_5906),
            ("xl/styles.xml", STYLES, 2235, 0x76B2_525A),
        ];
        for (name, part, len, crc) in whole {
            assert_eq!(part.len(), len, "{name}");
            assert_eq!(crate::zip::crc32(part.as_bytes()), crc, "{name}");
        }
        let fragments: [(&str, &str, usize, u32); 17] = [
            ("CONTENT_TYPES_HEAD", CONTENT_TYPES_HEAD, 414, 0xE25B_4644),
            (
                "CONTENT_TYPES_SHEET_OPEN",
                CONTENT_TYPES_SHEET_OPEN,
                40,
                0xC50A_5584,
            ),
            (
                "CONTENT_TYPES_SHEET_CLOSE",
                CONTENT_TYPES_SHEET_CLOSE,
                95,
                0xAC64_9AC9,
            ),
            ("CONTENT_TYPES_TAIL", CONTENT_TYPES_TAIL, 130, 0xC82B_7EA6),
            ("WORKBOOK_HEAD", WORKBOOK_HEAD, 218, 0xD3AA_947D),
            ("WORKBOOK_SHEET_OPEN", WORKBOOK_SHEET_OPEN, 13, 0x439D_849E),
            ("WORKBOOK_SHEET_ID", WORKBOOK_SHEET_ID, 11, 0xABA5_0681),
            ("WORKBOOK_SHEET_RID", WORKBOOK_SHEET_RID, 11, 0xD2A2_9207),
            ("WORKBOOK_SHEET_CLOSE", WORKBOOK_SHEET_CLOSE, 3, 0x17F5_6E5A),
            ("WORKBOOK_TAIL", WORKBOOK_TAIL, 20, 0x5222_CD6B),
            ("WORKBOOK_RELS_HEAD", WORKBOOK_RELS_HEAD, 140, 0x3587_245C),
            (
                "WORKBOOK_RELS_SHEET_OPEN",
                WORKBOOK_RELS_SHEET_OPEN,
                21,
                0x4E6A_D968,
            ),
            (
                "WORKBOOK_RELS_SHEET_TARGET",
                WORKBOOK_RELS_SHEET_TARGET,
                111,
                0xA82A_5703,
            ),
            (
                "WORKBOOK_RELS_SHEET_CLOSE",
                WORKBOOK_RELS_SHEET_CLOSE,
                7,
                0x1D79_5E1C,
            ),
            (
                "WORKBOOK_RELS_STYLES_OPEN",
                WORKBOOK_RELS_STYLES_OPEN,
                21,
                0x4E6A_D968,
            ),
            (
                "WORKBOOK_RELS_STYLES_CLOSE",
                WORKBOOK_RELS_STYLES_CLOSE,
                105,
                0x0BD6_B144,
            ),
            ("WORKBOOK_RELS_TAIL", WORKBOOK_RELS_TAIL, 16, 0x210F_1D67),
        ];
        for (name, part, len, crc) in fragments {
            assert_eq!(part.len(), len, "{name}");
            assert_eq!(crate::zip::crc32(part.as_bytes()), crc, "{name}");
        }
        // Cat-identity: one sheet named `data` reproduces the
        // pre-011 whole parts byte for byte.
        let ct: String = content_types_xml(1);
        assert_eq!(ct.len(), 680, "[Content_Types].xml");
        assert_eq!(crate::zip::crc32(ct.as_bytes()), 0xCC79_080A);
        let escaped: Vec<String> = vec![escape_sheet_name("data")];
        let wb: String = workbook_xml(&escaped);
        assert_eq!(wb.len(), 282, "xl/workbook.xml");
        assert_eq!(crate::zip::crc32(wb.as_bytes()), 0xD473_8FA2);
        let wr: String = workbook_rels_xml(1);
        assert_eq!(wr.len(), 424, "xl/_rels/workbook.xml.rels");
        assert_eq!(crate::zip::crc32(wr.as_bytes()), 0x8A96_42E0);
    }

    #[test]
    fn digit_widths_match_the_naive_spelling() {
        // The closed forms against the definition they replace, at
        // every decade boundary in the sheet-index range.
        let naive = |mut n: usize| {
            let mut w = 1;
            while n >= 10 {
                n /= 10;
                w += 1;
            }
            w
        };
        for n in [0, 1, 9, 10, 99, 100, 999, 1000, 65_535, 1_048_576] {
            assert_eq!(digit_width(n), naive(n), "width {n}");
            let summed: usize = (1..=n).map(naive).sum();
            assert_eq!(digit_width_sum(n), summed, "sum {n}");
        }
        assert_eq!(digit_width_sum(0), 0);
    }

    #[test]
    fn workbook_parts_cap_holds() {
        // Exact capacities: assembled length equals the cap spelling
        // for 1, 3, and 12 sheets (the last crosses a digit width).
        for count in [1_usize, 3, 12] {
            let escaped: Vec<String> = (1..=count)
                .map(|i| escape_sheet_name(&format!("Sheet{i}")))
                .collect();
            assert_eq!(content_types_xml(count).len(), content_types_cap(count));
            assert_eq!(workbook_xml(&escaped).len(), workbook_cap(&escaped));
            assert_eq!(workbook_rels_xml(count).len(), workbook_rels_cap(count));
        }
        let escaped_amp: Vec<String> = vec![escape_sheet_name("A&B")];
        assert_eq!(workbook_xml(&escaped_amp).len(), workbook_cap(&escaped_amp));
        assert!(workbook_xml(&escaped_amp).contains("A&amp;B"));
    }

    fn name_error(name: &str) -> String {
        let t: Vec<i64> = vec![1];
        let cols: Vec<(Column, ColumnData<'_>)> = vec![(
            Column {
                name: "t".to_string(),
                kind: Kind::I64,
                style: Style::default(),
            },
            ColumnData::I64(&t),
        )];
        build_workbook(&[DynamicSheet::known(name.to_string(), &cols)])
            .unwrap_err()
            .to_string()
    }

    fn names_error(names: &[&str]) -> String {
        let t: Vec<i64> = vec![1];
        let cols: Vec<(Column, ColumnData<'_>)> = vec![(
            Column {
                name: "t".to_string(),
                kind: Kind::I64,
                style: Style::default(),
            },
            ColumnData::I64(&t),
        )];
        let sheets: Vec<DynamicSheet<'_>> = names
            .iter()
            .map(|name| DynamicSheet::known(name.to_string(), &cols))
            .collect();
        build_workbook(&sheets).unwrap_err().to_string()
    }

    fn name_ok(name: &str) -> bool {
        let t: Vec<i64> = vec![1];
        let cols: Vec<(Column, ColumnData<'_>)> = vec![(
            Column {
                name: "t".to_string(),
                kind: Kind::I64,
                style: Style::default(),
            },
            ColumnData::I64(&t),
        )];
        build_workbook(&[DynamicSheet::known(name.to_string(), &cols)]).is_ok()
    }

    #[test]
    fn no_sheets_is_refused() {
        let empty: &[DynamicSheet<'_>] = &[];
        let err: String = build_workbook(empty).unwrap_err().to_string();
        assert!(err.contains("no sheets declared"), "{err}");
    }

    #[test]
    fn empty_sheet_name_is_refused() {
        let err: String = name_error("");
        assert!(err.contains(""), "{err}");
        assert!(err.contains("empty"), "{err}");
    }

    #[test]
    fn too_long_sheet_name_is_refused() {
        let name: String = "a".repeat(32);
        let err: String = name_error(&name);
        assert!(err.contains(&name), "{err}");
    }

    #[test]
    fn invalid_char_sheet_name_is_refused() {
        for bad in ['[', ']', ':', '*', '?', '/', '\\'] {
            let name: String = format!("a{bad}b");
            let err: String = name_error(&name);
            assert!(err.contains(&name), "{bad}: {err}");
        }
        let name: String = "a\u{1}b".to_string();
        let err: String = name_error(&name);
        assert!(err.contains(&name), "{err}");
    }

    #[test]
    fn apostrophe_sheet_name_is_refused() {
        for name in ["'data", "data'"] {
            let err: String = name_error(name);
            assert!(err.contains(name), "{err}");
        }
    }

    #[test]
    fn reserved_sheet_name_is_refused() {
        for name in ["History", "history", "HISTORY"] {
            let err: String = name_error(name);
            assert!(err.contains(name), "{err}");
        }
    }

    #[test]
    fn duplicate_sheet_name_is_refused() {
        let err: String = names_error(&["Data", "data"]);
        assert!(err.contains("data"), "{err}");
    }

    #[test]
    fn boundary_sheet_names_are_accepted() {
        assert!(name_ok(&"a".repeat(31)));
        assert!(name_ok("A&B"));
    }
}
