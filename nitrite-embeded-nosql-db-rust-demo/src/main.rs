use nitrite::doc;
use nitrite::filter::field;
use nitrite::nitrite::Nitrite;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Initialize an in-memory database
    // We pass None for the path and None for configuration to get default settings.
    let db = Nitrite::builder().open_or_create(None, None)?;
    // 2. Retrieve (or create) a collection named "users"
    let collection = db.collection("users")?;
    // 3. Insert documents using the doc! macro
    // The doc! macro allows you to construct flexible JSON-like documents.
    collection.insert(doc! {
        "name": "Jane Doe",
        "role": "engineer",
        "age": 28,
        "active": true,
        "skills": ["rust", "systems", "database"]
    })?;
    collection.insert(doc! {
        "name": "Bob Smith",
        "role": "manager",
        "age": 35,
        "active": false,
        "skills": ["agile", "planning"]
    })?;
    // 4. Query with the fluent filter API
    // Let's find all active users who are engineers
    println!("--- Searching for active engineers ---");
    let query = nitrite::filter::and(vec![field("role").eq("engineer"), field("active").eq(true)]);
    let cursor = collection.find(query)?;
    for doc_result in cursor {
        let doc = doc_result?;
        let name = doc
            .get("name")?
            .as_string()
            .cloned()
            .ok_or_else(|| std::io::Error::other("name is not a string"))?;
        let age = doc
            .get("age")?
            .as_i32()
            .copied()
            .ok_or_else(|| std::io::Error::other("age is not a 32-bit integer"))?;
        println!("Found: {} (Age: {})", name, age);
    }
    // 5. Clean up and close the database
    db.close()?;
    Ok(())
}
