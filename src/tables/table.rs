use postgres::{self, Row, types::FromSql, types::ToSql };
use std::convert::{TryFrom, TryInto};
use rust_decimal::Decimal;
use super::column::*;
use super::column::try_into::*;
use super::column::from::*;
use super::nullable_column::*;
use rusqlite::{self, Rows};
use super::csv;
use std::fmt::{self, Display};
use std::string::ToString;
use num_traits::cast::ToPrimitive;

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
        let mut query = format!("create table {} (", name);
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

    pub fn shape(&self) -> (usize, usize) {
        (self.nrows, self.cols.len())
    }

    pub fn get_columns<'a>(&'a self, ixs : &[usize]) -> Columns<'a> {
        let mut cols = Columns::new();
        for ix in ixs.iter() {
            match (self.names.get(*ix), self.cols.get(*ix)) {
                (Some(name), Some(col)) => { cols = cols.take_and_push(name, col); },
                _ => println!("Column not found at index {}", ix)
            }
        }
        cols
    }

    pub fn get_column<'a>(&'a self, ix : usize) -> Option<&'a Column> {
        self.cols.get(ix)
    }

    pub fn names(&self) -> Vec<String> {
        self.names.clone()
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
#[derive(Clone, Debug)]
pub struct Columns<'a> {
    names : Vec<&'a str>,
    cols : Vec<&'a Column>
}

impl<'a> Columns<'a> {

    pub fn new() -> Self {
        Self{ names : Vec::new(), cols: Vec::new() }
    }

    pub fn take_and_push(mut self, name : &'a str, col : &'a Column) -> Self {
        self.names.push(name);
        self.cols.push(col);
        self
    }

    pub fn take_and_extend(mut self, cols : Columns<'a>) -> Self {
        self.names.extend(cols.names);
        self.cols.extend(cols.cols);
        self
    }

    pub fn names(&'a self) -> &'a [&'a str] {
        &self.names[..]
    }

    pub fn get(&'a self, ix : usize) -> Option<&'a Column> {
        self.cols.get(ix).map(|c| *c)
    }

    // TODO move this to the implementation of try_into(.)
    /// Tries to retrieve a cloned copy from a column, performing any valid
    /// upcasts required to retrieve a f64 numeric type.
    pub fn try_numeric(&'a self, ix : usize) -> Option<Vec<f64>>
        where
            Column : TryInto<Vec<f64>,Error=&'static str>
    {
        if let Some(dbl) = self.try_access::<f64>(ix) {
            return Some(dbl);
        }
        if let Some(float) = self.try_access::<f32>(ix) {
            let cvt : Vec<f64> = float.iter().map(|f| *f as f64).collect();
            return Some(cvt);
        }
        if let Some(short) = self.try_access::<i16>(ix) {
            let cvt : Vec<f64> = short.iter().map(|s| *s as f64).collect();
            return Some(cvt);
        }
        if let Some(int) = self.try_access::<i32>(ix) {
            let cvt : Vec<f64> = int.iter().map(|i| *i as f64).collect();
            return Some(cvt);
        }
        if let Some(int) = self.try_access::<i32>(ix) {
            let cvt : Vec<f64> = int.iter().map(|i| *i as f64).collect();
            return Some(cvt);
        }
        if let Some(uint) = self.try_access::<u32>(ix) {
            let cvt : Vec<f64> = uint.iter().map(|u| *u as f64).collect();
            return Some(cvt);
        }
        if let Some(long) = self.try_access::<i64>(ix) {
            let cvt : Vec<f64> = long.iter().map(|l| *l as f64).collect();
            return Some(cvt);
        }
        if let Some(dec) = self.try_access::<Decimal>(ix) {
            let mut cvt : Vec<f64> = Vec::new();
            for d in dec.iter() {
                if let Some(f) = d.to_f64() {
                    cvt.push(f);
                } else {
                    println!("Invalid decimal conversion");
                    return None;
                }
            }
            return Some(cvt);
        }
        println!("Invalid column conversion");
        None
    }

    pub fn try_access<T>(&'a self, ix : usize) -> Option<Vec<T>>
        where
            Column : TryInto<Vec<T>, Error=&'static str>
    {
        if let Some(c) = self.get(ix) {
            let v : Result<Vec<T>,_> = c.clone().try_into();
            match v {
                Ok(c) => { Some(c) },
                Err(_) => { /*println!("{}", e);*/ None }
            }
        } else {
            println!("Invalid column index");
            None
        }
    }

}


