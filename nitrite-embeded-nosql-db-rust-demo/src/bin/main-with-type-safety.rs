use nitrite::nitrite::Nitrite;
use nitrite::repository::ObjectRepository;
use nitrite_derive::{Convertible, NitriteEntity};

// Annotate your struct to define its collection name and ID field
#[derive(Default, Convertible, NitriteEntity)]
#[entity(name = "users", id(field = "id"))]
pub struct User {
    pub id: i64,
    pub name: String,
    pub email: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db = Nitrite::builder().open_or_create(None, None)?;

    // Get a type-safe object repository
    let repo: ObjectRepository<User> = db.repository::<User>()?;

    // Insert a native Rust struct directly
    repo.insert(User {
        id: 42,
        name: "Alice Jones".to_string(),
        email: "alice@example.com".to_string(),
    })?;

    // Fetch the object directly by its ID
    if let Some(user) = repo.get_by_id(&42)? {
        println!("Retrieved user: {} ({})", user.name, user.email);
    }
    Ok(())
}
