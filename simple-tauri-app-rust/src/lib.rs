use tauri::async_runtime::Mutex;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(Mutex::new(User {
            id: 0,
            username: "".to_string(),
            password: "".to_string(),
            email: "".to_string(),
        }))
        .invoke_handler(tauri::generate_handler![get_user, login])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[tauri::command]
async fn get_user(state: tauri::State<'_, Mutex<User>>) -> Result<User, ()> {
    Ok(state.lock().await.clone())
}

#[tauri::command]
async fn login(
    state: tauri::State<'_, Mutex<User>>,
    username: String,
    password: String,
    email: String,
) -> Result<bool, ()> {
    *state.lock().await = User {
        id: 1,
        username,
        password,
        email,
    };

    Ok(true)
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
struct User {
    id: u32,
    username: String,
    password: String,
    email: String,
}
