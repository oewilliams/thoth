//! Thoth: a fast xlsx writer from a declared schema.
//!
//! Named for Thoth, the Egyptian scribe of records and measurement.
//! The schema is declared, never sniffed: each column's type is imposed
//! once at the boundary, and the emitters below are monomorphic after
//! that. No per-cell type switch, no shared-string hash: style is
//! declared once per column, like kind.

mod macros;

pub mod contract;
pub mod emit;
pub mod static_parts;
pub mod workbook_parts;
pub mod xlsx;
pub(crate) mod zip;
