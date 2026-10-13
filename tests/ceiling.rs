//! The ceiling verdict is witnessed, not projected: a workbook
//! whose `sheet_cap` projection passes the zip32 ceiling builds
//! when its actual stored length fits it.

use thoth::contract::{Column, DynamicSheet, Kind, Style, sheet_cap, sheet_letters};
use thoth::xlsx::{ColumnData, build_workbook};

/// Three sheets at the work-spec shape (8,880 rows x 1,298 float
/// columns, simplified to float columns throughout), blank-heavy:
/// `NaN` cells spell a few bytes as a blank against the same
/// 327-byte `f64` cap price, so the projection passes the ceiling
/// while the actual archive is a few hundred MB. The assertion
/// below is the relationship the test relies on, so a cap change
/// that silently untests it fails loudly.
#[test]
fn spec_sheet_shape_builds_under_the_ceiling_its_projection_exceeds() {
    let rows: usize = 8_880;
    let ncols: usize = 1_298;
    let blank: Vec<f64> = vec![f64::NAN; rows];
    let zeros: Vec<f64> = vec![0.0_f64; rows];
    let cols: Vec<(Column, ColumnData<'_>)> = (0..ncols)
        .map(|i| {
            let data: &[f64] = if i == 0 { &zeros } else { &blank };
            (
                Column {
                    name: format!("c{i}"),
                    kind: Kind::F64,
                    style: Style::default(),
                },
                ColumnData::F64(data),
            )
        })
        .collect();
    let sheets: Vec<DynamicSheet<'_>> = ["SchCF", "SchCF_ED", "SchCF_GU"]
        .iter()
        .map(|name| DynamicSheet::known(name.to_string(), &cols))
        .collect();
    let ceiling: usize = u32::MAX as usize;
    let mut projection: usize = 0;
    for sheet in &sheets {
        let letters: Vec<String> = sheet_letters(sheet);
        projection += sheet_cap(sheet, rows, &letters);
    }
    let archive: Vec<u8> = build_workbook(&sheets).unwrap();
    assert!(
        projection > ceiling,
        "projection {projection} must exceed ceiling {ceiling} for this test to test anything"
    );
    assert!(
        archive.len() <= ceiling,
        "actual {} must fit ceiling {ceiling}",
        archive.len()
    );
    assert!(archive.len() < projection);
}
