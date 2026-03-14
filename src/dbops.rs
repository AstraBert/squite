use indexmap::IndexMap;
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

pub fn execute_sqlite_statement(query: &str, db_file: &str) -> Result<usize> {
    if query.to_lowercase().starts_with("select") {
        return Err(rusqlite::Error::ExecuteReturnedResults);
    }
    let conn = Connection::open(db_file)?;
    let mut stmt = conn.prepare(query)?;
    let result = stmt.execute([])?;
    return Ok(result);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execute_sqlite_query_one_column() {
        let result = execute_sqlite_query("SELECT DISTINCT user FROM test;", "testfiles/test.db")
            .expect("Should be able to execute query");
        assert!(result.contains_key("user"));
        let users = result
            .get("user")
            .expect("User should be available within the result");
        assert_eq!(users.len(), 3);
    }

    #[test]
    fn test_execute_sqlite_query_many_columns() {
        let result = execute_sqlite_query("SELECT * FROM test;", "testfiles/test.db")
            .expect("Should be able to execute query");
        assert!(result.contains_key("user"));
        assert!(result.contains_key("message"));
        assert!(result.contains_key("id"));
        let users = result
            .get("user")
            .expect("'user' should be available within the result");
        assert_eq!(users.len(), 9);
        let messages = result
            .get("message")
            .expect("'message' should be available within the result");
        assert_eq!(messages.len(), 9);
        let idxs = result
            .get("id")
            .expect("'message' should be available within the result");
        assert_eq!(idxs.len(), 9);
    }

    #[test]
    fn test_execute_sqlite_query_db_not_exists() {
        let result = execute_sqlite_query("SELECT * FROM test;", "testfiles/no_test.db"); // this creates no_test.db, but the test table does not exist
        assert!(result.is_err_and(|e| e.to_string() == "no such table: test".to_string()));
    }

    #[test]
    fn test_execute_sqlite_query_non_readonly_query() {
        let result = execute_sqlite_query(
            "INSERT INTO test (user, message) VALUES ('alice', 'Another question?');",
            "testfiles/test.db",
        ); // this fails as an invalid query
        assert!(result.is_err_and(|e| e == rusqlite::Error::InvalidQuery));
    }
}
