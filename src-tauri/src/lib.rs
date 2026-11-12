mod clipboard;
mod filesystem;
mod rename;

use tauri::Manager;
use tauri_plugin_opener::OpenerExt;

#[tauri::command]
async fn read_file_clipboard(
    app: tauri::AppHandle,
) -> Result<Option<clipboard::FileClipboard>, String> {
    blocking(move || app.state::<clipboard::SystemClipboard>().read()).await
}

#[tauri::command]
async fn write_file_clipboard(
    app: tauri::AppHandle,
    operation: clipboard::FileClipboard,
) -> Result<(), String> {
    blocking(move || app.state::<clipboard::SystemClipboard>().write(operation)).await
}

#[tauri::command]
async fn clear_file_clipboard(
    app: tauri::AppHandle,
    expected: clipboard::FileClipboard,
) -> Result<(), String> {
    blocking(move || app.state::<clipboard::SystemClipboard>().clear(expected)).await
}

#[tauri::command]
async fn write_clipboard_text(app: tauri::AppHandle, text: String) -> Result<(), String> {
    blocking(move || app.state::<clipboard::SystemClipboard>().text(text)).await
}

#[tauri::command]
async fn start_file_drag(window: tauri::WebviewWindow, paths: Vec<String>) -> Result<(), String> {
    let paths = blocking(move || clipboard::drag_paths(paths)).await?;
    let (sender, receiver) = tauri::async_runtime::channel(1);
    let main_window = window.clone();
    window
        .run_on_main_thread(move || {
            let result = perform_file_drag(main_window, paths);
            let _ = sender.try_send(result);
        })
        .map_err(|e| e.to_string())?;
    let mut receiver = receiver;
    receiver.recv().await.ok_or("系统拖放任务已结束")?
}

// Native drag APIs must run on the window's main thread.
fn perform_file_drag(
    window: tauri::WebviewWindow,
    paths: Vec<std::path::PathBuf>,
) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    let handle = window.gtk_window().map_err(|e| e.to_string())?;
    #[cfg(not(target_os = "linux"))]
    let handle = window;
    drag::start_drag(
        &handle,
        drag::DragItem::Files(paths),
        drag::Image::Raw(include_bytes!("../icons/32x32.png").to_vec()),
        |_, _| {},
        drag::Options::default(),
    )
    .map_err(|e| format!("无法开始系统拖放：{e}"))
}

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
async fn read_image(path: String) -> Result<tauri::ipc::Response, String> {
    blocking(move || filesystem::read_image(&path))
        .await
        .map(tauri::ipc::Response::new)
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
        .manage(clipboard::SystemClipboard::default())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_app_info,
            read_directory,
            read_image,
            create_folder,
            rename_entry,
            transfer_entries,
            trash_entries,
            open_entry,
            read_file_clipboard,
            write_file_clipboard,
            clear_file_clipboard,
            write_clipboard_text,
            start_file_drag
        ])
        .run(tauri::generate_context!())
        .expect("无法启动 Pudding");
}
