//! Frozen workbook skeleton (golden bytes); the pin test in
//! `crate::xlsx` trips on any edit.
//!
//! `RELS` and `STYLES` name no sheet and stay frozen whole. The three
//! sheet-naming parts are frozen fragments: concatenating head, one
//! per-sheet fragment per sheet, and tail for the single sheet `data`
//! reproduces the pre-011 whole-part bytes exactly (cat-identity is
//! pinned in `crate::xlsx` tests).

pub(crate) const CONTENT_TYPES_HEAD: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/>"#;

pub(crate) const CONTENT_TYPES_SHEET_OPEN: &str = r#"<Override PartName="/xl/worksheets/sheet"#;

pub(crate) const CONTENT_TYPES_SHEET_CLOSE: &str = r#".xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>"#;

pub(crate) const CONTENT_TYPES_TAIL: &str = r#"<Override PartName="/xl/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml"/></Types>"#;

pub(crate) const RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#;

pub(crate) const WORKBOOK_HEAD: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets>"#;

pub(crate) const WORKBOOK_SHEET_OPEN: &str = r#"<sheet name=""#;

pub(crate) const WORKBOOK_SHEET_ID: &str = r#"" sheetId=""#;

pub(crate) const WORKBOOK_SHEET_RID: &str = r#"" r:id="rId"#;

pub(crate) const WORKBOOK_SHEET_CLOSE: &str = r#""/>"#;

pub(crate) const WORKBOOK_TAIL: &str = r#"</sheets></workbook>"#;

pub(crate) const WORKBOOK_RELS_HEAD: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">"#;

pub(crate) const WORKBOOK_RELS_SHEET_OPEN: &str = r#"<Relationship Id="rId"#;

pub(crate) const WORKBOOK_RELS_SHEET_TARGET: &str = r#"" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet"#;

pub(crate) const WORKBOOK_RELS_SHEET_CLOSE: &str = r#".xml"/>"#;

pub(crate) const WORKBOOK_RELS_STYLES_OPEN: &str = r#"<Relationship Id="rId"#;

pub(crate) const WORKBOOK_RELS_STYLES_CLOSE: &str = r#"" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/>"#;

pub(crate) const WORKBOOK_RELS_TAIL: &str = r#"</Relationships>"#;

pub(crate) const STYLES: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><fonts count="2"><font><sz val="11"/><name val="Calibri"/></font><font><b/><sz val="11"/><name val="Calibri"/></font></fonts><fills count="8"><fill><patternFill patternType="none"/></fill><fill><patternFill patternType="gray125"/></fill><fill><patternFill patternType="solid"><fgColor rgb="FFFF0000"/></patternFill></fill><fill><patternFill patternType="solid"><fgColor rgb="FF00B050"/></patternFill></fill><fill><patternFill patternType="solid"><fgColor rgb="FFFFC000"/></patternFill></fill><fill><patternFill patternType="solid"><fgColor rgb="FF0070C0"/></patternFill></fill><fill><patternFill patternType="solid"><fgColor rgb="FF7030A0"/></patternFill></fill><fill><patternFill patternType="solid"><fgColor rgb="FFBFBFBF"/></patternFill></fill></fills><borders><border><left/><right/><top/><bottom/><diagonal/></border></borders><cellStyleXfs count="1"><xf numFmtId="0" fontId="0" fillId="0" borderId="0"/></cellStyleXfs><cellXfs count="14"><xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0"/><xf numFmtId="0" fontId="1" fillId="0" borderId="0" xfId="0" applyFont="1"/><xf numFmtId="0" fontId="0" fillId="2" borderId="0" xfId="0" applyFill="1"/><xf numFmtId="0" fontId="1" fillId="2" borderId="0" xfId="0" applyFont="1" applyFill="1"/><xf numFmtId="0" fontId="0" fillId="3" borderId="0" xfId="0" applyFill="1"/><xf numFmtId="0" fontId="1" fillId="3" borderId="0" xfId="0" applyFont="1" applyFill="1"/><xf numFmtId="0" fontId="0" fillId="4" borderId="0" xfId="0" applyFill="1"/><xf numFmtId="0" fontId="1" fillId="4" borderId="0" xfId="0" applyFont="1" applyFill="1"/><xf numFmtId="0" fontId="0" fillId="5" borderId="0" xfId="0" applyFill="1"/><xf numFmtId="0" fontId="1" fillId="5" borderId="0" xfId="0" applyFont="1" applyFill="1"/><xf numFmtId="0" fontId="0" fillId="6" borderId="0" xfId="0" applyFill="1"/><xf numFmtId="0" fontId="1" fillId="6" borderId="0" xfId="0" applyFont="1" applyFill="1"/><xf numFmtId="0" fontId="0" fillId="7" borderId="0" xfId="0" applyFill="1"/><xf numFmtId="0" fontId="1" fillId="7" borderId="0" xfId="0" applyFont="1" applyFill="1"/></cellXfs></styleSheet>"#;
