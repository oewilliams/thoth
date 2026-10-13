//! Declaration sugar over the template types. Each macro expands
//! to the plain value a user could write by hand; dropping the
//! sugar changes no bytes.

/// One style value from bracket tokens (helper for the column
/// macros). The style grammar is `bold`, one palette colour
/// (`red`, `green`, `amber`, `blue`, `purple`, `grey`), or a
/// colour and `bold` in either order; anything else fails here.
#[doc(hidden)]
#[macro_export]
macro_rules! __thoth_style {
    (bold) => {
        $crate::contract::Style {
            bold: true,
            colour: None,
        }
    };
    (red) => {
        $crate::contract::Style {
            bold: false,
            colour: Some($crate::contract::Colour::Red),
        }
    };
    (green) => {
        $crate::contract::Style {
            bold: false,
            colour: Some($crate::contract::Colour::Green),
        }
    };
    (amber) => {
        $crate::contract::Style {
            bold: false,
            colour: Some($crate::contract::Colour::Amber),
        }
    };
    (blue) => {
        $crate::contract::Style {
            bold: false,
            colour: Some($crate::contract::Colour::Blue),
        }
    };
    (purple) => {
        $crate::contract::Style {
            bold: false,
            colour: Some($crate::contract::Colour::Purple),
        }
    };
    (grey) => {
        $crate::contract::Style {
            bold: false,
            colour: Some($crate::contract::Colour::Grey),
        }
    };
    (red, bold) => {
        $crate::contract::Style {
            bold: true,
            colour: Some($crate::contract::Colour::Red),
        }
    };
    (bold, red) => {
        $crate::contract::Style {
            bold: true,
            colour: Some($crate::contract::Colour::Red),
        }
    };
    (green, bold) => {
        $crate::contract::Style {
            bold: true,
            colour: Some($crate::contract::Colour::Green),
        }
    };
    (bold, green) => {
        $crate::contract::Style {
            bold: true,
            colour: Some($crate::contract::Colour::Green),
        }
    };
    (amber, bold) => {
        $crate::contract::Style {
            bold: true,
            colour: Some($crate::contract::Colour::Amber),
        }
    };
    (bold, amber) => {
        $crate::contract::Style {
            bold: true,
            colour: Some($crate::contract::Colour::Amber),
        }
    };
    (blue, bold) => {
        $crate::contract::Style {
            bold: true,
            colour: Some($crate::contract::Colour::Blue),
        }
    };
    (bold, blue) => {
        $crate::contract::Style {
            bold: true,
            colour: Some($crate::contract::Colour::Blue),
        }
    };
    (purple, bold) => {
        $crate::contract::Style {
            bold: true,
            colour: Some($crate::contract::Colour::Purple),
        }
    };
    (bold, purple) => {
        $crate::contract::Style {
            bold: true,
            colour: Some($crate::contract::Colour::Purple),
        }
    };
    (grey, bold) => {
        $crate::contract::Style {
            bold: true,
            colour: Some($crate::contract::Colour::Grey),
        }
    };
    (bold, grey) => {
        $crate::contract::Style {
            bold: true,
            colour: Some($crate::contract::Colour::Grey),
        }
    };
    ($($style:tt)*) => {
        compile_error!(
            "column style grammar: expected `bold`, one colour (`red`, `green`, `amber`, `blue`, `purple`, `grey`), or a colour and `bold` in either order"
        )
    };
}

/// One static column from its kind token (helper for the sheet macros).
#[doc(hidden)]
#[macro_export]
macro_rules! __thoth_static_column {
    (i64 $name:literal => $data:expr) => {
        $crate::contract::StaticColumn::I64 {
            name: $name,
            style: $crate::contract::Style::default(),
            data: $data,
        }
    };
    (f64 $name:literal => $data:expr) => {
        $crate::contract::StaticColumn::F64 {
            name: $name,
            style: $crate::contract::Style::default(),
            data: $data,
        }
    };
    (str $name:literal => $data:expr) => {
        $crate::contract::StaticColumn::Str {
            name: $name,
            style: $crate::contract::Style::default(),
            data: $data,
        }
    };
    (bool $name:literal => $data:expr) => {
        $crate::contract::StaticColumn::Bool {
            name: $name,
            style: $crate::contract::Style::default(),
            data: $data,
        }
    };
    (i64 [ $($style:ident),+ ] $name:literal => $data:expr) => {
        $crate::contract::StaticColumn::I64 {
            name: $name,
            style: $crate::__thoth_style!($($style),+),
            data: $data,
        }
    };
    (f64 [ $($style:ident),+ ] $name:literal => $data:expr) => {
        $crate::contract::StaticColumn::F64 {
            name: $name,
            style: $crate::__thoth_style!($($style),+),
            data: $data,
        }
    };
    (str [ $($style:ident),+ ] $name:literal => $data:expr) => {
        $crate::contract::StaticColumn::Str {
            name: $name,
            style: $crate::__thoth_style!($($style),+),
            data: $data,
        }
    };
    (bool [ $($style:ident),+ ] $name:literal => $data:expr) => {
        $crate::contract::StaticColumn::Bool {
            name: $name,
            style: $crate::__thoth_style!($($style),+),
            data: $data,
        }
    };
    ($kind:ident [ $($style:tt)* ] $name:literal => $data:expr) => {
        compile_error!(
            "column style grammar: expected `bold`, one colour (`red`, `green`, `amber`, `blue`, `purple`, `grey`), or a colour and `bold` in either order"
        )
    };
    ($kind:ident $name:literal => $data:expr) => {
        compile_error!("column kind grammar: expected `i64`, `f64`, `str`, or `bool`")
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
                style: $crate::contract::Style::default(),
            },
            $crate::xlsx::ColumnData::I64($data),
        )
    };
    (f64 $name:literal => $data:expr) => {
        (
            $crate::contract::Column {
                name: $name.to_string(),
                kind: $crate::contract::Kind::F64,
                style: $crate::contract::Style::default(),
            },
            $crate::xlsx::ColumnData::F64($data),
        )
    };
    (str $name:literal => $data:expr) => {
        (
            $crate::contract::Column {
                name: $name.to_string(),
                kind: $crate::contract::Kind::Str,
                style: $crate::contract::Style::default(),
            },
            $crate::xlsx::ColumnData::Str($data),
        )
    };
    (bool $name:literal => $data:expr) => {
        (
            $crate::contract::Column {
                name: $name.to_string(),
                kind: $crate::contract::Kind::Bool,
                style: $crate::contract::Style::default(),
            },
            $crate::xlsx::ColumnData::Bool($data),
        )
    };
    (i64 [ $($style:ident),+ ] $name:literal => $data:expr) => {
        (
            $crate::contract::Column {
                name: $name.to_string(),
                kind: $crate::contract::Kind::I64,
                style: $crate::__thoth_style!($($style),+),
            },
            $crate::xlsx::ColumnData::I64($data),
        )
    };
    (f64 [ $($style:ident),+ ] $name:literal => $data:expr) => {
        (
            $crate::contract::Column {
                name: $name.to_string(),
                kind: $crate::contract::Kind::F64,
                style: $crate::__thoth_style!($($style),+),
            },
            $crate::xlsx::ColumnData::F64($data),
        )
    };
    (str [ $($style:ident),+ ] $name:literal => $data:expr) => {
        (
            $crate::contract::Column {
                name: $name.to_string(),
                kind: $crate::contract::Kind::Str,
                style: $crate::__thoth_style!($($style),+),
            },
            $crate::xlsx::ColumnData::Str($data),
        )
    };
    (bool [ $($style:ident),+ ] $name:literal => $data:expr) => {
        (
            $crate::contract::Column {
                name: $name.to_string(),
                kind: $crate::contract::Kind::Bool,
                style: $crate::__thoth_style!($($style),+),
            },
            $crate::xlsx::ColumnData::Bool($data),
        )
    };
    ($kind:ident [ $($style:tt)* ] $name:literal => $data:expr) => {
        compile_error!(
            "column style grammar: expected `bold`, one colour (`red`, `green`, `amber`, `blue`, `purple`, `grey`), or a colour and `bold` in either order"
        )
    };
    ($kind:ident $name:literal => $data:expr) => {
        compile_error!("column kind grammar: expected `i64`, `f64`, `str`, or `bool`")
    };
}

/// A column array to bind before building a dynamic sheet:
///
/// ```rust
/// let t = [1i64, 2];
/// let cols = thoth::columns!(i64 "t" => &t);
/// ```
///
/// Style brackets declare the column style:
///
/// ```rust
/// let t = [1i64, 2];
/// let cols = thoth::columns!(i64 [red, bold] "t" => &t);
/// ```
#[macro_export]
macro_rules! columns {
    ($($kind:ident $( [ $($style:tt)* ] )? $name:literal => $data:expr),+ $(,)?) => {
        [ $( $crate::__thoth_column!($kind $( [ $($style)* ] )? $name => $data) ),+ ]
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
/// Style brackets declare the column style, as a hand-written
/// `Style` value would:
///
/// ```rust
/// let sheet = thoth::static_sheet!("fixed",
///     i64 [bold] "t" => [100, 200, 300],
///     str [blue] "n" => [Some("a"), None, Some("")],
/// );
/// ```
///
/// An unknown colour fails at compile time naming the style grammar:
///
/// ```compile_fail
/// let _sheet = thoth::static_sheet!("x", str [pink] "n" => [Some("a")]);
/// ```
///
/// Two colours fail the same way:
///
/// ```compile_fail
/// let _sheet = thoth::static_sheet!("x", str [red, blue] "n" => [Some("a")]);
/// ```
///
/// A repeated `bold` fails the same way:
///
/// ```compile_fail
/// let _sheet = thoth::static_sheet!("x", str [bold, bold] "n" => [Some("a")]);
/// ```
///
/// A non-identifier token fails the same way:
///
/// ```compile_fail
/// let _sheet = thoth::static_sheet!("x", str [123] "n" => [Some("a")]);
/// ```
#[macro_export]
macro_rules! static_sheet {
    ($name:literal, $($kind:ident $( [ $($style:tt)* ] )? $col:literal => $data:expr),+ $(,)?) => {
        $crate::contract::StaticSheet::new(
            $name,
            [ $( $crate::__thoth_static_column!($kind $( [ $($style)* ] )? $col => $data) ),+ ],
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
    (unknown_rows $name:literal, $($kind:ident $( [ $($style:tt)* ] )? $col:literal => $data:expr),+ $(,)?) => {
        $crate::contract::DynamicSheet::unknown_rows(
            $name.to_string(),
            &[ $( $crate::__thoth_column!($kind $( [ $($style)* ] )? $col => $data) ),+ ],
        )
    };
    (unknown_columns $name:literal, $($kind:ident $( [ $($style:tt)* ] )? $col:literal => $data:expr),+ $(,)?) => {
        $crate::contract::DynamicSheet::unknown_columns(
            $name.to_string(),
            &[ $( $crate::__thoth_column!($kind $( [ $($style)* ] )? $col => $data) ),+ ],
        )
    };
    ($name:literal, $($kind:ident $( [ $($style:tt)* ] )? $col:literal => $data:expr),+ $(,)?) => {
        $crate::contract::DynamicSheet::known(
            $name.to_string(),
            &[ $( $crate::__thoth_column!($kind $( [ $($style)* ] )? $col => $data) ),+ ],
        )
    };
}
