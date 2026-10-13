//! Declaration sugar over the template types. Each macro expands
//! to the plain value a user could write by hand; dropping the
//! sugar changes no bytes.

/// One static column from its kind token (helper for the sheet macros).
#[doc(hidden)]
#[macro_export]
macro_rules! __thoth_static_column {
    (i64 $name:literal => $data:expr) => {
        $crate::contract::StaticColumn::I64 {
            name: $name,
            data: $data,
        }
    };
    (f64 $name:literal => $data:expr) => {
        $crate::contract::StaticColumn::F64 {
            name: $name,
            data: $data,
        }
    };
    (str $name:literal => $data:expr) => {
        $crate::contract::StaticColumn::Str {
            name: $name,
            data: $data,
        }
    };
    (bool $name:literal => $data:expr) => {
        $crate::contract::StaticColumn::Bool {
            name: $name,
            data: $data,
        }
    };
    ($kind:ident [ $($style:tt)* ] $name:literal => $data:expr) => {
        compile_error!("column styles are reserved syntax and are not implemented yet")
    };
}

/// One dynamic column pair from its kind token (helper).
#[doc(hidden)]
#[macro_export]
macro_rules! __thoth_column {
    (i64 $name:literal => $data:expr) => {
        (
            $crate::contract::Column {
                name: $name.to_string(),
                kind: $crate::contract::Kind::I64,
            },
            $crate::xlsx::ColumnData::I64($data),
        )
    };
    (f64 $name:literal => $data:expr) => {
        (
            $crate::contract::Column {
                name: $name.to_string(),
                kind: $crate::contract::Kind::F64,
            },
            $crate::xlsx::ColumnData::F64($data),
        )
    };
    (str $name:literal => $data:expr) => {
        (
            $crate::contract::Column {
                name: $name.to_string(),
                kind: $crate::contract::Kind::Str,
            },
            $crate::xlsx::ColumnData::Str($data),
        )
    };
    (bool $name:literal => $data:expr) => {
        (
            $crate::contract::Column {
                name: $name.to_string(),
                kind: $crate::contract::Kind::Bool,
            },
            $crate::xlsx::ColumnData::Bool($data),
        )
    };
    ($kind:ident [ $($style:tt)* ] $name:literal => $data:expr) => {
        compile_error!("column styles are reserved syntax and are not implemented yet")
    };
}

/// A column array to bind before building a dynamic sheet:
///
/// ```rust
/// let t = [1i64, 2];
/// let cols = thoth::columns!(i64 "t" => &t);
/// ```
#[macro_export]
macro_rules! columns {
    ($($kind:ident $( [ $($style:ident),* ] )? $name:literal => $data:expr),+ $(,)?) => {
        [ $( $crate::__thoth_column!($kind $( [ $($style),* ] )? $name => $data) ),+ ]
    };
}

/// Declare a fixed-shape sheet; column and row counts are inferred.
///
/// ```rust
/// let sheet = thoth::static_sheet!("fixed",
///     i64 "t"  => [100, 200, 300],
///     f64 "px" => [10.5, 11.0, 12.25],
///     str "n"  => [Some("a"), None, Some("")],
///     bool "ok" => [Some(true), Some(false), None],
/// );
/// ```
///
/// Style brackets stay reserved and fail at compile time:
///
/// ```compile_fail
/// let _sheet = thoth::static_sheet!("x", str [bold] "n" => [Some("a")]);
/// ```
#[macro_export]
macro_rules! static_sheet {
    ($name:literal, $($kind:ident $( [ $($style:ident),* ] )? $col:literal => $data:expr),+ $(,)?) => {
        $crate::contract::StaticSheet::new(
            $name,
            [ $( $crate::__thoth_static_column!($kind $( [ $($style),* ] )? $col => $data) ),+ ],
        )
    };
}

/// Declare a dynamic sheet from borrowed slices. The columns are a
/// temporary array, so use this spelling inside the build call, or
/// bind the array first with [`columns!`] for a stored sheet.
///
/// ```rust
/// let t = [1i64, 2];
/// let px = [1.5f64, 2.5];
/// let bytes = thoth::xlsx::build_workbook(&[
///     thoth::dynamic_sheet!("data", i64 "t" => &t, f64 "px" => &px),
/// ]).unwrap();
/// ```
///
/// Unknown dimensions stay explicit: `dynamic_sheet!(unknown_rows
/// "data", ...)` or `unknown_columns` in place of the sheet name.
#[macro_export]
macro_rules! dynamic_sheet {
    (unknown_rows $name:literal, $($kind:ident $( [ $($style:ident),* ] )? $col:literal => $data:expr),+ $(,)?) => {
        $crate::contract::DynamicSheet::unknown_rows(
            $name.to_string(),
            &[ $( $crate::__thoth_column!($kind $( [ $($style),* ] )? $col => $data) ),+ ],
        )
    };
    (unknown_columns $name:literal, $($kind:ident $( [ $($style:ident),* ] )? $col:literal => $data:expr),+ $(,)?) => {
        $crate::contract::DynamicSheet::unknown_columns(
            $name.to_string(),
            &[ $( $crate::__thoth_column!($kind $( [ $($style),* ] )? $col => $data) ),+ ],
        )
    };
    ($name:literal, $($kind:ident $( [ $($style:ident),* ] )? $col:literal => $data:expr),+ $(,)?) => {
        $crate::contract::DynamicSheet::known(
            $name.to_string(),
            &[ $( $crate::__thoth_column!($kind $( [ $($style),* ] )? $col => $data) ),+ ],
        )
    };
}
