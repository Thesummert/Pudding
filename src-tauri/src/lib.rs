mod clipboard;
mod filesystem;
mod open_requests;
mod rename;
mod search;
mod session;

use tauri::{Emitter, Manager};
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

#[derive(serde::Serialize)]
struct DropModifiers {
    ctrl: bool,
    shift: bool,
}

#[tauri::command]
async fn drop_modifiers(window: tauri::WebviewWindow) -> Result<DropModifiers, String> {
    let (sender, mut receiver) = tauri::async_runtime::channel(1);
    window
        .run_on_main_thread(move || {
            let _ = sender.try_send(read_drop_modifiers());
        })
        .map_err(|e| e.to_string())?;
    receiver
        .recv()
        .await
        .ok_or("无法读取修饰键状态".to_string())?
}

fn read_drop_modifiers() -> Result<DropModifiers, String> {
    #[cfg(target_os = "macos")]
    {
        use objc2_app_kit::{NSEvent, NSEventModifierFlags};
        let flags = NSEvent::modifierFlags_class();
        Ok(DropModifiers {
            ctrl: flags.contains(NSEventModifierFlags::Control),
            shift: flags.contains(NSEventModifierFlags::Shift),
        })
    }
    #[cfg(target_os = "windows")]
    {
        use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
            GetAsyncKeyState, VK_CONTROL, VK_SHIFT,
        };
        Ok(DropModifiers {
            ctrl: unsafe { GetAsyncKeyState(VK_CONTROL as i32) } < 0,
            shift: unsafe { GetAsyncKeyState(VK_SHIFT as i32) } < 0,
        })
    }
    #[cfg(target_os = "linux")]
    {
        let display = gtk::gdk::Display::default().ok_or("无法读取显示器")?;
        let keymap = gtk::gdk::Keymap::for_display(&display).ok_or("无法读取键盘状态")?;
        let state = keymap.modifier_state();
        Ok(DropModifiers {
            ctrl: state & gtk::gdk::ModifierType::CONTROL_MASK.bits() != 0,
            shift: state & gtk::gdk::ModifierType::SHIFT_MASK.bits() != 0,
        })
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        Err("无法读取键盘状态".into())
    }
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
fn take_open_directories(state: tauri::State<'_, open_requests::OpenRequests>) -> Vec<String> {
    state.take()
}

#[tauri::command]
fn startup_directory(state: tauri::State<'_, session::Session>) -> Option<String> {
    state.startup_path()
}

#[tauri::command]
fn set_window_directory(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, session::Session>,
    path: String,
) {
    state.set_directory(window.label(), path);
}

#[tauri::command]
async fn read_directory(path: String) -> Result<filesystem::DirectoryListing, String> {
    blocking(move || filesystem::read_directory(&path)).await
}

#[tauri::command]
async fn prepare_video_preview(app: tauri::AppHandle, path: String) -> Result<String, String> {
    let path = blocking(move || filesystem::video_preview_path(&path)).await?;
    app.asset_protocol_scope()
        .allow_file(&path)
        .map_err(|e| format!("无法预览视频：{e}"))?;
    Ok(path)
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
async fn delete_entries(paths: Vec<String>) -> Result<(), String> {
    blocking(move || filesystem::delete_entries(paths)).await
}

#[tauri::command]
async fn open_entry(app: tauri::AppHandle, path: String) -> Result<(), String> {
    let path = blocking(move || filesystem::openable_path(&path)).await?;
    app.opener()
        .open_path(path, None::<&str>)
        .map_err(|e| format!("无法用默认应用打开：{e}"))
}

#[tauri::command]
async fn watch_directory(window: tauri::WebviewWindow, path: String) -> Result<(), String> {
    blocking(move || search::watch(&window.state::<search::SearchState>(), window.clone(), path))
        .await
}

#[tauri::command]
async fn search_files(
    window: tauri::WebviewWindow,
    path: String,
    query: String,
    hidden: bool,
    search_content: bool,
    request_id: u32,
) -> Result<Vec<filesystem::FileEntry>, String> {
    blocking(move || {
        search::search(
            &window.state::<search::SearchState>(),
            path,
            query,
            hidden,
            search_content,
            |count| {
                let _ = window.emit("search-progress", (request_id, count));
            },
        )
    })
    .await
}

#[tauri::command]
async fn open_directory_window(app: tauri::AppHandle, path: String) -> Result<(), String> {
    let path = blocking(move || filesystem::resolve_directory(&path)).await?;
    let query = url::form_urlencoded::Serializer::new(String::new())
        .append_pair("path", &path.to_string_lossy())
        .finish();
    let label = format!(
        "folder-{}",
        std::sync::atomic::AtomicU64::fetch_add(
            &WINDOW_ID,
            1,
            std::sync::atomic::Ordering::Relaxed
        )
    );
    tauri::WebviewWindowBuilder::new(
        &app,
        label,
        tauri::WebviewUrl::App(format!("index.html?{query}").into()),
    )
    .title("Pudding · 文件管理器")
    .inner_size(1280.0, 820.0)
    .min_inner_size(860.0, 580.0)
    .center()
    .build()
    .map_err(|e| format!("无法打开新窗口：{e}"))?;
    Ok(())
}

static WINDOW_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(clipboard::SystemClipboard::default())
        .manage(search::SearchState::default())
        .manage(open_requests::OpenRequests::default())
        .setup(|app| {
            app.state::<open_requests::OpenRequests>()
                .push(open_requests::arguments(std::env::args_os().skip(1)));
            let session = app
                .path()
                .app_data_dir()
                .ok()
                .and_then(|path| session::Session::new(&path).ok())
                .unwrap_or_else(session::Session::disabled);
            app.manage(session);
            Ok(())
        })
        .on_window_event(|window, event| {
            if matches!(event, tauri::WindowEvent::Destroyed) {
                search::unwatch(&window.state::<search::SearchState>(), window.label());
                window
                    .state::<session::Session>()
                    .close_window(window.label());
            }
        })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_app_info,
            startup_directory,
            take_open_directories,
            set_window_directory,
            watch_directory,
            open_directory_window,
            search_files,
            read_directory,
            read_image,
            prepare_video_preview,
            create_folder,
            rename_entry,
            transfer_entries,
            trash_entries,
            delete_entries,
            open_entry,
            read_file_clipboard,
            write_file_clipboard,
            clear_file_clipboard,
            write_clipboard_text,
            start_file_drag,
            drop_modifiers
        ])
        .build(tauri::generate_context!())
        .expect("无法启动 Pudding")
        .run({
            #[cfg(target_os = "macos")]
            let mut ready = false;
            move |app, event| {
                #[cfg(target_os = "macos")]
                if matches!(event, tauri::RunEvent::Ready) {
                    ready = true;
                }
                #[cfg(target_os = "macos")]
                if let tauri::RunEvent::Opened { urls } = event {
                    if app
                        .state::<open_requests::OpenRequests>()
                        .push(urls.iter().filter_map(open_requests::local_url))
                    {
                        // The OS may deliver an open event before Tauri creates its
                        // configured main window. Its frontend will drain the queue.
                        if !ready {
                            return;
                        }
                        let windows = app.webview_windows();
                        if let Some(window) =
                            windows.get("main").or_else(|| windows.values().next())
                        {
                            let _ = window.unminimize();
                            let _ = window.show();
                            let _ = window.set_focus();
                            let _ = app.emit_to(
                                tauri::EventTarget::webview(window.label()),
                                "open-directories",
                                (),
                            );
                        } else {
                            // A background macOS app may have no remaining windows.
                            if let Err(error) = tauri::WebviewWindowBuilder::new(
                                app,
                                "main",
                                tauri::WebviewUrl::App("index.html".into()),
                            )
                            .title("Pudding · 文件管理器")
                            .inner_size(1280.0, 820.0)
                            .min_inner_size(860.0, 580.0)
                            .center()
                            .build()
                            {
                                eprintln!("无法打开窗口：{error}");
                            }
                        }
                    }
                }
                #[cfg(not(target_os = "macos"))]
                let _ = (app, event);
            }
        });
}
