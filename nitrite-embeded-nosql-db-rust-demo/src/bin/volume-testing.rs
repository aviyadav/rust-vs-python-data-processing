use nitrite::filter::field;
use nitrite::index::non_unique_index;
use nitrite::nitrite::Nitrite;
use nitrite::repository::ObjectRepository;
use nitrite_derive::{Convertible, NitriteEntity};
use nitrite_fjall_adapter::FjallModule;
use std::path::Path;
use std::time::{Duration, Instant};

const DEFAULT_USER_COUNT: usize = 1_000_000;
const DEFAULT_BATCH_SIZE: usize = 100_000;
const DB_PATH: &str = "target/volume-testing-db";

type AppResult<T> = Result<T, Box<dyn std::error::Error>>;

#[derive(Default, Convertible, NitriteEntity)]
#[entity(name = "users", id(field = "id"))]
pub struct User {
    pub id: i64,
    pub name: String,
    pub email: String,
}

struct SearchTarget {
    id: i64,
    name: String,
    email: String,
}

fn create_random_user(id: i64) -> User {
    let name_suffix = rand::random::<u64>();
    let email_suffix = rand::random::<u64>();

    User {
        id,
        name: format!("User-{name_suffix:016x}"),
        email: format!("user-{email_suffix:016x}@example.com"),
    }
}

fn generate_and_insert_users(
    db: &Nitrite,
    repo: &ObjectRepository<User>,
    count: usize,
    batch_size: usize,
) -> AppResult<SearchTarget> {
    let target_id = i64::try_from((count / 2).max(1))?;
    let mut target = None;
    let mut generation_time = Duration::ZERO;
    let mut insertion_time = Duration::ZERO;
    let mut inserted = 0;
    let progress_interval = (count / 100).max(batch_size);
    let mut next_progress = progress_interval;

    while inserted < count {
        let current_batch_size = batch_size.min(count - inserted);
        let first_id = i64::try_from(inserted + 1)?;

        let generation_started_at = Instant::now();
        let users = (0..current_batch_size)
            .map(|offset| create_random_user(first_id + offset as i64))
            .collect::<Vec<_>>();
        generation_time += generation_started_at.elapsed();

        if target.is_none()
            && let Some(user) = users.iter().find(|user| user.id == target_id)
        {
            target = Some(SearchTarget {
                id: user.id,
                name: user.name.clone(),
                email: user.email.clone(),
            });
        }

        let insertion_started_at = Instant::now();
        repo.insert_many(users)?;
        db.commit()?;
        insertion_time += insertion_started_at.elapsed();

        inserted += current_batch_size;
        if inserted >= next_progress || inserted == count {
            println!(
                "Inserted {inserted}/{count} users ({:.1}%)",
                inserted as f64 / count as f64 * 100.0
            );
            next_progress = next_progress.saturating_add(progress_interval);
        }
    }

    println!("Generated {count} random users in {generation_time:?}");
    println!("Inserted {count} users in {insertion_time:?}");

    target.ok_or_else(|| std::io::Error::other("no generated user available for searches").into())
}

fn search_by_id(repo: &ObjectRepository<User>, id: i64) -> AppResult<Option<User>> {
    let started_at = Instant::now();
    let result = repo.get_by_id(&id)?;
    let elapsed = started_at.elapsed();

    match &result {
        Some(user) => println!(
            "ID search found {} ({}) in {:?}",
            user.name, user.email, elapsed
        ),
        None => println!("ID search found no user for {id} in {elapsed:?}"),
    }

    Ok(result)
}

fn search_by_name(repo: &ObjectRepository<User>, name: &str) -> AppResult<Option<User>> {
    let started_at = Instant::now();
    let mut cursor = repo.find(field("name").eq(name))?;
    let result = cursor.next().transpose()?;
    let elapsed = started_at.elapsed();

    match &result {
        Some(user) => println!(
            "Name search found {} ({}) in {:?}",
            user.name, user.email, elapsed
        ),
        None => println!("Name search found no user for {name} in {elapsed:?}"),
    }

    Ok(result)
}

fn search_by_email(repo: &ObjectRepository<User>, email: &str) -> AppResult<Option<User>> {
    let started_at = Instant::now();
    let mut cursor = repo.find(field("email").eq(email))?;
    let result = cursor.next().transpose()?;
    let elapsed = started_at.elapsed();

    match &result {
        Some(user) => println!(
            "Email search found {} ({}) in {:?}",
            user.name, user.email, elapsed
        ),
        None => println!("Email search found no user for {email} in {elapsed:?}"),
    }

    Ok(result)
}

fn parse_positive_argument(position: usize, default: usize, name: &str) -> AppResult<usize> {
    let Some(value) = std::env::args().nth(position) else {
        return Ok(default);
    };

    let value = value.parse::<usize>()?;
    if value == 0 {
        return Err(std::io::Error::other(format!("{name} must be greater than zero")).into());
    }
    Ok(value)
}

fn main() -> AppResult<()> {
    let user_count = parse_positive_argument(1, DEFAULT_USER_COUNT, "user count")?;
    let batch_size = parse_positive_argument(2, DEFAULT_BATCH_SIZE, "batch size")?;
    i64::try_from(user_count)?;

    if Path::new(DB_PATH).exists() {
        std::fs::remove_dir_all(DB_PATH)?;
    }

    let storage = FjallModule::with_config()
        .low_memory_preset()
        .db_path(DB_PATH)
        .build();
    let db = Nitrite::builder()
        .load_module(storage)
        .open_or_create(None, None)?;
    let repo: ObjectRepository<User> = db.repository::<User>()?;
    repo.create_index(vec!["name"], &non_unique_index())?;
    repo.create_index(vec!["email"], &non_unique_index())?;

    println!("Using disk-backed storage at {DB_PATH} with batches of {batch_size} users");
    let target = generate_and_insert_users(&db, &repo, user_count, batch_size)?;

    search_by_id(&repo, target.id)?;
    search_by_name(&repo, &target.name)?;
    search_by_email(&repo, &target.email)?;

    drop(repo);
    db.close()?;
    std::fs::remove_dir_all(DB_PATH)?;
    println!("Removed temporary database at {DB_PATH}");
    Ok(())
}
