use serde::Serialize;
use std::path::Path;

#[derive(Serialize)]
pub struct Application {
    id: String,
    name: String,
}

fn checked_file(path: &str) -> Result<String, String> {
    let path = crate::filesystem::openable_path(path)?;
    if !Path::new(&path).is_file() {
        return Err("请选择文件".into());
    }
    Ok(path)
}

#[cfg(target_os = "linux")]
fn linux_apps(path: &str) -> Vec<gtk::gio::AppInfo> {
    let (kind, _) = gtk::gio::content_type_guess(Some(path), &[]);
    gtk::gio::AppInfo::recommended_for_type(&kind)
}

pub fn list(path: &str) -> Result<Vec<Application>, String> {
    let path = checked_file(path)?;
    #[cfg(target_os = "linux")]
    {
        use gtk::gio::prelude::*;
        let mut seen = std::collections::HashSet::new();
        Ok(linux_apps(&path)
            .into_iter()
            .filter_map(|app| {
                let id = app.id()?.to_string();
                if !seen.insert(id.clone()) {
                    return None;
                }
                Some(Application {
                    id,
                    name: app.display_name().to_string(),
                })
            })
            .collect())
    }
    #[cfg(target_os = "macos")]
    {
        use objc2_app_kit::NSWorkspace;
        use objc2_foundation::{NSString, NSURL};
        let file = NSURL::fileURLWithPath(&NSString::from_str(&path));
        let urls = NSWorkspace::sharedWorkspace().URLsForApplicationsToOpenURL(&file);
        Ok(urls
            .iter()
            .filter_map(|url| {
                let id = url.path()?.to_string();
                let name = Path::new(&id).file_stem()?.to_string_lossy().into_owned();
                Some(Application { id, name })
            })
            .collect())
    }
    #[cfg(target_os = "windows")]
    {
        windows_apps(&path)
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        let _ = path;
        Ok(Vec::new())
    }
}

#[cfg(target_os = "windows")]
fn windows_apps(path: &str) -> Result<Vec<Application>, String> {
    let extension = Path::new(path)
        .extension()
        .and_then(|v| v.to_str())
        .unwrap_or("");
    if extension.is_empty() {
        return Ok(Vec::new());
    }
    // Query file associations without inserting the file name into a command line.
    let script = r#"
[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new()
$ext = '.' + $env:PUDDING_OPEN_EXTENSION
$classes = [Microsoft.Win32.Registry]::ClassesRoot
$user = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey('Software\Microsoft\Windows\CurrentVersion\Explorer\FileExts\' + $ext)
$ids = @()
if ($user) {
  $choice = $user.OpenSubKey('UserChoice'); if ($choice) { $ids += $choice.GetValue('ProgId') }
  $progids = $user.OpenSubKey('OpenWithProgids'); if ($progids) { $ids += $progids.GetValueNames() }
  $apps = $user.OpenSubKey('OpenWithList'); if ($apps) { foreach ($n in $apps.GetValueNames()) { if ($n -ne 'MRUList') { $ids += 'Applications\' + $apps.GetValue($n) } } }
}
$key = $classes.OpenSubKey($ext)
if ($key) { $ids += $key.GetValue(''); $open = $key.OpenSubKey('OpenWithProgids'); if ($open) { $ids += $open.GetValueNames() } }
foreach ($id in ($ids | Where-Object { $_ } | Select-Object -Unique)) {
  $key = $classes.OpenSubKey($id + '\shell\open\command'); if (!$key) { continue }
  $command = [Environment]::ExpandEnvironmentVariables([string]$key.GetValue(''))
  if ($command -match '^\s*"([^"]+\.exe)"' -or $command -match '^\s*([^"\s]+\.exe)') {
    $exe = $Matches[1]; if ([IO.File]::Exists($exe)) { [Console]::WriteLine($exe) }
  }
}
"#;
    let output = std::process::Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .env("PUDDING_OPEN_EXTENSION", extension)
        .output()
        .map_err(|e| format!("无法读取系统应用：{e}"))?;
    if !output.status.success() {
        return Err("无法读取系统应用关联".into());
    }
    let mut result: Vec<Application> = Vec::new();
    for id in String::from_utf8_lossy(&output.stdout).lines() {
        let id = id.trim();
        if id.is_empty() || result.iter().any(|app| app.id.eq_ignore_ascii_case(id)) {
            continue;
        }
        let name = Path::new(id)
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        result.push(Application {
            id: id.to_owned(),
            name,
        });
    }
    Ok(result)
}

pub fn open_recommended(path: &str, id: &str) -> Result<(), String> {
    let path = checked_file(path)?;
    #[cfg(target_os = "linux")]
    {
        use gtk::gio::{self, prelude::*};
        let app = linux_apps(&path)
            .into_iter()
            .find(|app| app.id().as_deref() == Some(id))
            .ok_or("应用已不可用，请重新选择")?;
        app.launch(
            &[gio::File::for_path(&path)],
            None::<&gio::AppLaunchContext>,
        )
        .map_err(|e| format!("无法打开文件：{e}"))
    }
    #[cfg(not(target_os = "linux"))]
    {
        if !list(&path)?.iter().any(|app| app.id == id) {
            return Err("应用已不可用，请重新选择".into());
        }
        launch(&path, Path::new(id))
    }
}

pub fn choose(path: &str) -> Result<(), String> {
    let path = checked_file(path)?;
    let dialog = rfd::FileDialog::new().set_title("选择打开文件的应用");
    #[cfg(target_os = "macos")]
    let dialog = dialog
        .set_directory("/Applications")
        .add_filter("应用程序", &["app"]);
    #[cfg(target_os = "windows")]
    let dialog = dialog.add_filter("应用程序", &["exe"]);
    #[cfg(target_os = "linux")]
    let dialog = dialog.set_directory("/usr/bin");
    if let Some(application) = dialog.pick_file() {
        launch(&path, &application)?;
    }
    Ok(())
}

fn launch(path: &str, application: &Path) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    let mut command = {
        if application.extension().and_then(|s| s.to_str()) != Some("app") || !application.is_dir()
        {
            return Err("请选择应用程序（.app）".into());
        }
        let mut command = std::process::Command::new("open");
        command.arg("-a").arg(application).arg("--");
        command
    };
    #[cfg(not(target_os = "macos"))]
    let mut command = std::process::Command::new(application);
    command.arg(path);
    #[cfg(target_os = "macos")]
    if !command
        .status()
        .map_err(|e| format!("无法打开文件：{e}"))?
        .success()
    {
        return Err("无法使用所选应用打开文件".into());
    }
    #[cfg(not(target_os = "macos"))]
    command.spawn().map_err(|e| format!("无法打开文件：{e}"))?;
    Ok(())
}
