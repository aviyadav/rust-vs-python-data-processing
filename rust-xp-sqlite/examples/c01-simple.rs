use pretty_sqlite::print_table;
use rusqlite::Connection;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // println!("Hello, world!");

    // -- Memory SQLite
    // let conn = Connection::open("my-db.db3")?;
    let conn = Connection::open_in_memory()?;

    // -- Create schema
    conn.execute(
        "CREATE TABLE IF NOT EXISTS person (
            id      INTEGER PRIMARY KEY,
            name    TEXT NOT NULL,
            yob     INTEGER, -- year of birth
            data    BLOB
        ) STRICT",
        (), // empty list of parameters.
    )?;

    // -- Insert data
    // OK in `STRICT` mode
    conn.execute(
        "INSERT INTO person (name, yob)
            VALUES (?, ?)",
        ("John", &1980), // can use "1980" as well
    )?;

    // -- Query data
    let select_sql = "SELECT person.id, person.name, person.yob
                        FROM person
                        WHERE yob > :yob";
    let mut stmt = conn.prepare(select_sql)?;
    let mut rows = stmt.query(&[(":yob", &1900)])?;

    while let Some(row) = rows.next()? {
        let name: String = row.get(1)?;
        println!("->> name: {name}");
        println!("->> row: {row:?}");
    }

    print_table(&conn, "person")?;

    Ok(())
}
