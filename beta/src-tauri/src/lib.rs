use indexmap::IndexMap;
use rusqlite::types::ValueRef;
use rusqlite::{Connection, Result}; // need to swap values with std::result::Result with type String so Tauri does not complain
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

#[tauri::command]
pub fn execute_sqlite_query(query: &str, db_file: &str) -> Result<IndexMap<String, Vec<Value>>> {
    if !query.to_lowercase().starts_with("select") {
        return Err(rusqlite::Error::InvalidQuery);
    }
    let conn = Connection::open(db_file)?;
    let mut stmt = conn.prepare(query)?;
    let column_names: Vec<String> = stmt.column_names().iter().map(|s| s.to_string()).collect();
    let results: Vec<Map<String, Value>> = stmt
        .query_map([], |row| row_to_json(row, &column_names))?
        .collect::<Result<Vec<_>>>()?;
    let mut table: IndexMap<String, Vec<Value>> = IndexMap::new();
    for column in column_names {
        let mut col_contents: Vec<Value> = vec![];
        for result in &results {
            if let Some(r) = result.get(&column) {
                col_contents.push(r.clone());
            }
        }
        table.insert(column, col_contents);
    }

    Ok(table)
}

#[tauri::command]
pub fn execute_sqlite_statement(query: &str, db_file: &str) -> Result<usize> {
    if query.to_lowercase().starts_with("select") {
        return Err(rusqlite::Error::ExecuteReturnedResults);
    }
    let conn = Connection::open(db_file)?;
    let mut stmt = conn.prepare(query)?;
    let result = stmt.execute([])?;
    Ok(result)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            execute_sqlite_query,
            execute_sqlite_statement
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
