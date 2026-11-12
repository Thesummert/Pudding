mod filesystem;
mod rename;

use tauri_plugin_opener::OpenerExt;

async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(work)
        .await
        .map_err(|e| format!("文件操作任务失败：{e}"))?
}

#[tauri::command]
async fn get_app_info() -> Result<filesystem::AppInfo, String> {
    blocking(filesystem::app_info).await
}

#[tauri::command]
async fn read_directory(path: String) -> Result<filesystem::DirectoryListing, String> {
    blocking(move || filesystem::read_directory(&path)).await
}

#[tauri::command]
async fn create_folder(parent: String, name: String) -> Result<String, String> {
    blocking(move || filesystem::create_folder(&parent, &name)).await
}

#[tauri::command]
async fn rename_entry(path: String, name: String) -> Result<String, String> {
    blocking(move || filesystem::rename_entry(&path, &name)).await
}

#[tauri::command]
async fn transfer_entries(
    paths: Vec<String>,
    destination: String,
    mode: String,
) -> Result<Vec<String>, String> {
    blocking(move || filesystem::transfer_entries(paths, &destination, &mode)).await
}

#[tauri::command]
async fn trash_entries(paths: Vec<String>) -> Result<(), String> {
    blocking(move || filesystem::trash_entries(paths)).await
}

#[tauri::command]
async fn open_entry(app: tauri::AppHandle, path: String) -> Result<(), String> {
    let path = blocking(move || filesystem::openable_path(&path)).await?;
    app.opener()
        .open_path(path, None::<&str>)
        .map_err(|e| format!("无法用默认应用打开：{e}"))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_app_info,
            read_directory,
            create_folder,
            rename_entry,
            transfer_entries,
            trash_entries,
            open_entry
        ])
        .run(tauri::generate_context!())
        .expect("无法启动 Pudding");
}
