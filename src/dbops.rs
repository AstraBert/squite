use std::collections::HashMap;

use rusqlite::types::ValueRef;
use rusqlite::{Connection, Result};
use serde_json::{Map, Value};

fn row_to_json(row: &rusqlite::Row, column_names: &[String]) -> Result<Map<String, Value>> {
    let mut map = Map::new();
    for (i, name) in column_names.iter().enumerate() {
        let val = match row.get_ref(i)? {
            ValueRef::Null => Value::Null,
            ValueRef::Integer(n) => Value::from(n),
            ValueRef::Real(f) => Value::from(f),
            ValueRef::Text(s) => Value::from(std::str::from_utf8(s).unwrap()),
            ValueRef::Blob(b) => Value::from(b),
        };
        map.insert(name.clone(), val);
    }
    Ok(map)
}

pub fn execute_sqlite_query(query: &str, db_file: &str) -> Result<HashMap<String, Vec<Value>>> {
    if !query.to_lowercase().starts_with("select") {
        return Err(rusqlite::Error::InvalidQuery);
    }
    let conn = Connection::open(db_file)?;
    let mut stmt = conn.prepare(query)?;
    let column_names: Vec<String> = stmt.column_names().iter().map(|s| s.to_string()).collect();
    let results: Vec<Map<String, Value>> = stmt
        .query_map([], |row| row_to_json(row, &column_names))?
        .collect::<Result<Vec<_>>>()?;
    let mut table: HashMap<String, Vec<Value>> = HashMap::new();
    for column in column_names {
        let mut col_contents: Vec<Value> = vec![];
        for result in &results {
            match result.get(&column) {
                Some(r) => {
                    col_contents.push(r.clone());
                }
                None => {}
            }
        }
        table.insert(column, col_contents);
    }

    Ok(table)
}
