//! The schema contract: one declared column per Excel column, in order.
//!
//! The kind selects the emitter once per column, never per cell. Ingress
//! is borrowed (see [`crate::xlsx::ColumnData`]); this module owns only
//! the small column metadata, not the bulk data.

/// Column kind: the one decision made per column, never per cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    I64,
    F64,
}

/// One declared column: Excel order is declaration order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Column {
    pub name: String,
    pub kind: Kind,
}

/// The i64 null: absence lives in the value's own bits.
pub const NULL_I64: i64 = i64::MIN;
