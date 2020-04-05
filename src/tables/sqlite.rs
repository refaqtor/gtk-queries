use rusqlite::{self, Rows, types::Value };
use std::convert::{TryFrom, TryInto};
use rust_decimal::Decimal;
use super::column::*;
use super::nullable_column::*;
use std::fmt::Display;
use super::table::*;

#[derive(Debug)]
enum SqliteColumn {
    I64(Vec<Option<i64>>),
    F64(Vec<Option<f64>>),
    Str(Vec<Option<String>>),
    Bytes(Vec<Option<Vec<u8>>>)
}

impl SqliteColumn {

    fn new(decl_type : &str) -> Result<Self, &'static str> {
        match decl_type {
            "integer" => Ok(SqliteColumn::I64(Vec::new())),
            "real" => Ok(SqliteColumn::F64(Vec::new())),
            "text" => Ok(SqliteColumn::Str(Vec::new())),
            "blob" => Ok(SqliteColumn::Bytes(Vec::new())),
            _ => { println!(" Informed type: {} ", decl_type); Err("Invalid column type") }
        }
    }

    fn try_append(&mut self, value : Value) -> Result<(), &'static str> {
        match self {
            Self::I64(ref mut v) => {
                match value {
                    Value::Integer(i) => v.push(Some(i)),
                    Value::Null => v.push(None),
                    _ => {
                        println!("Column type: {:?}", self);
                        println!("Error parsing to: {}", value.data_type());
                        return Err("Invalid type");
                    }
                }
            },
            Self::F64(ref mut v) => {
                match value {
                    Value::Real(r) => v.push(Some(r)),
                    Value::Null => v.push(None),
                    _ => {
                        println!("Column type: {:?}", self);
                        println!("Error parsing to: {}", value.data_type());
                        return Err("Invalid type");
                    }
                }
            },
            Self::Str(ref mut v) => {
                match value {
                    Value::Text(t) => v.push(Some(t)),
                    Value::Null => v.push(None),
                    _ => {
                        println!("Column type: {:?}", self);
                        println!("Error parsing to: {}", value.data_type());
                        return Err("Invalid type");
                    }
                }
            },
            Self::Bytes(ref mut v) => {
                match value {
                    Value::Blob(b) => v.push(Some(b)),
                    Value::Null => v.push(None),
                    _ => {
                        println!("Column type: {:?}", self);
                        println!("Error parsing to: {}", value.data_type());
                        return Err("Invalid type");
                    }
                }
            }
        }
        Ok(())
    }

}

impl From<SqliteColumn> for NullableColumn
    where
        NullableColumn : From<Vec<Option<i64>>>,
        NullableColumn : From<Vec<Option<f64>>>,
        NullableColumn : From<Vec<Option<String>>>,
        NullableColumn : From<Vec<Option<Vec<u8>>>>,
{
    fn from(col: SqliteColumn) -> Self {
        match col {
            SqliteColumn::I64(v) => v.into(),
            SqliteColumn::F64(v) => v.into(),
            SqliteColumn::Str(v) => v.into(),
            SqliteColumn::Bytes(v) => v.into()
        }
    }
}

pub fn build_table_from_sqlite(mut rows : rusqlite::Rows) -> Result<Table, &'static str>
    where
        NullableColumn : From<Vec<Option<i64>>>,
        NullableColumn : From<Vec<Option<f64>>>,
        NullableColumn : From<Vec<Option<String>>>,
        NullableColumn : From<Vec<Option<Vec<u8>>>>,
{
    let cols = rows.columns().ok_or("No columns available")?;
    let col_names = rows.column_names().ok_or("No columns available")?;
    let col_types : Vec<Option<&str>> = cols.iter().map(|c| c.decl_type()).collect();
    let names : Vec<_> = col_names.iter().map(|c| c.to_string()).collect();
    if names.len() == 0 {
        return Err("No columns available");
    }
    let mut sqlite_cols : Vec<SqliteColumn> = Vec::new();
    for (i, ty) in col_types.iter().enumerate() {
        if let Some(t) = ty {
            sqlite_cols.push(SqliteColumn::new(t)?);
        } else {
            println!("Type unknown at column: {}", i);
        }
    }
    while let Ok(row) = rows.next() {
        match row {
            Some(r) => {
                for (i, _) in names.iter().enumerate() {
                    let value = r.get::<usize, rusqlite::types::Value>(i)
                        .unwrap_or(rusqlite::types::Value::Null);
                    sqlite_cols[i].try_append(value)?;
                }
            },
            None => { break; }
        }
    }
    let mut null_cols : Vec<NullableColumn> = sqlite_cols.drain(0..names.len())
        .map(|c| c.into() ).collect();
    if null_cols.len() == 0 {
        return Err("Too few columns");
    }
    let cols : Vec<Column> = null_cols.drain(0..names.len())
        .map(|nc| nc.to_column()).collect();
    Ok(Table::new(names, cols)?)
}

