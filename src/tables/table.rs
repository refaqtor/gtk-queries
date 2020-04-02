use postgres::{self, Row, types::FromSql, types::ToSql };
use std::convert::{TryFrom, TryInto};
use rust_decimal::Decimal;
use super::column::*;
use super::nullable_column::*;
use rusqlite::{self, Rows};
use super::csv;
use std::fmt::{self, Display};
use std::string::ToString;

/// Data-owning structure that encapsulate named columns.
/// Implementation guarantees all columns are of the same size.
#[derive(Debug, Clone)]
pub struct Table {
    names : Vec<String>,
    cols : Vec<Column>,
    nrows : usize
}

impl Table {

    pub fn new(names : Vec<String>, cols : Vec<Column>) -> Result<Self, &'static str> {
        if names.len() != cols.len() {
            return Err("Differing number of names and columns");
        }
        let nrows = if let Some(col0) = cols.get(0) {
            col0.len()
        } else {
            return Err("No column zero");
        };
        for c in cols.iter().skip(1) {
            if c.len() != nrows {
                return Err("Number of rows mismatch at table creation");
            }
        }
        Ok(Self { names, cols, nrows })
    }

    pub fn new_from_text(
        source : String
    ) -> Result<Self, &'static str> {
        match csv::parse_csv_as_text_cols(&source.clone()) {
            Ok(mut cols) => {
                let mut parsed_cols = Vec::new();
                let mut names = Vec::new();
                for (name, values) in cols.drain(0..) {
                    let mut parsed_int = Vec::new();
                    let mut parsed_float = Vec::new();
                    let mut all_int = true;
                    let mut all_float = true;
                    for s in values.iter() {
                        if all_int {
                            if let Ok(int) = s.parse::<i64>() {
                                parsed_int.push(int);
                            } else {
                                all_int = false;
                            }
                        }
                        if all_float {
                            if let Ok(float) = s.parse::<f64>() {
                                parsed_float.push(float);
                            } else {
                                all_float = false;
                            }
                        }
                    }
                    match (all_int, all_float) {
                        (true, _) => parsed_cols.push(Column::I64(parsed_int)),
                        (false, true) => parsed_cols.push(Column::F64(parsed_float)),
                        _ => parsed_cols.push(Column::Str(values))
                    }
                    names.push(name);
                }
                Ok(Table::new(names, parsed_cols)?)
            },
            Err(e) => {
                println!("Error when creating table from text source : {}", e);
                Err("Could not parse CSV content")
            }
        }
    }

    pub fn flatten<'a>(&'a self) -> Result<Vec<Vec<&'a (dyn ToSql+Sync)>>, &'static str> {
        let dyn_cols : Vec<_> = self.cols.iter().map(|c| c.ref_content()).collect();
        if dyn_cols.len() == 0 {
            return Err("Query result is empty");
        }
        let n = dyn_cols[0].len();
        let mut dyn_rows = Vec::new();
        for r in 0..n {
            let mut dyn_r = Vec::new();
            for c in dyn_cols.iter() {
                dyn_r.push(c[r]);
            }
            dyn_rows.push(dyn_r);
        }
        Ok(dyn_rows)
    }

    pub fn text_rows(&self) -> Vec<Vec<String>> {
        let txt_cols : Vec<_> = self.cols.iter().map(|c| c.display_content()).collect();
        if txt_cols.len() == 0 {
            Vec::new()
        } else {
            let mut rows = Vec::new();
            let header = self.names.clone();
            rows.push(header);
            let n = txt_cols[0].len();
            for i in 0..n {
                let mut row = Vec::new();
                for c_txt in &txt_cols {
                    row.push(c_txt[i].clone());
                }
                rows.push(row);
            }
            rows
        }
    }

    /// Returns a SQL string (valid for SQlite3/PostgreSQL subset)
    /// which will contain both the table creation and data insertion
    /// commands. Binary columns are created but will hold NULL. Fails
    /// if table is not named.
    /// TODO check if SQL is valid (maybe external to the struct). SQL can
    /// be invalid if there are reserved keywords as column names.
    pub fn sql_string(&self, name : &str) -> Option<String> {
        self.sql_table_creation(name).map(|mut creation| {
            creation += &self.sql_table_insertion(name);
            creation
        })
    }

    pub fn sql_types(&self) -> Vec<String> {
        self.cols.iter().map(|c| c.sqlite3_type().to_string()).collect()
    }

    pub fn sql_table_creation(&self, name : &str) -> Option<String> {
        let mut query = format!("create table if not exists {} (", name);
        for (i, (name, col)) in self.names.iter().zip(self.cols.iter()).enumerate() {
            let name = match name.chars().find(|c| *c == ' ') {
                Some(_) => String::from("\"") + &name[..] + "\"",
                None => name.clone()
            };
            query += &format!("{} {}", name, col.sqlite3_type());
            if i < self.cols.len() - 1 {
                query += ","
            } else {
                query += ");\n"
            }
        }
        Some(query)
    }

    /// Always successful, but query might be empty if there is no data on the columns.
    pub fn sql_table_insertion(&self, name : &str) -> String {
        let mut q = String::new();
        let mut content = self.text_rows();
        if self.cols.len() <= 1 {
            return q;
        }
        content.remove(0);
        let types = self.sql_types();
        for line in content.iter() {
            q += &format!("insert into {} values (", name);
            for (i, (f, t)) in line.iter().zip(types.iter()).enumerate() {
                match &t[..] {
                    "text" => {
                        let quoted = String::from("'") + f + "'";
                        q += &quoted
                    },
                    _ => { q +=&f }
                };
                if i < line.len() - 1 {
                    q += ","
                } else {
                    q += ");\n"
                }
            }
        }
        q
    }

}

impl Display for Table {

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut content = String::new();
        for row in self.text_rows() {
            for (i, field) in row.iter().enumerate() {
                if i >= 1 {
                    content += ",";
                }
                content += &field[..];
            }
            content += "\n";
        }
        write!(f, "{}", content)
    }

}

/// Referential structure that encapsulate iteration over named columns.
/// Since columns might have different tables as their source,
/// there is no guarantee columns will have the same size.
pub struct Columns<'a> {
    names : &'a [&'a str],
    cols : &'a [Column]
}

