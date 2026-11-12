use serde::Serialize;
use std::path::Path;

#[derive(Clone, Serialize)]
pub struct Volume {
    pub id: String,
    pub name: String,
    pub path: String,
    pub ejectable: bool,
}

fn volume(path: &str, name: &str, ejectable: bool) -> Volume {
    Volume {
        id: path.into(),
        name: name.into(),
        path: path.into(),
        ejectable,
    }
}

pub fn fallback() -> Vec<Volume> {
    #[cfg(windows)]
    {
        Vec::new()
    }
    #[cfg(not(windows))]
    {
        vec![volume("/", "文件系统", false)]
    }
}

#[cfg(target_os = "macos")]
pub fn list() -> Result<Vec<Volume>, String> {
    objc2::rc::autoreleasepool(|_| list_macos())
}

#[cfg(target_os = "macos")]
fn list_macos() -> Result<Vec<Volume>, String> {
    use objc2_foundation::{
        NSFileManager, NSNumber, NSURLResourceKey, NSURLVolumeIsEjectableKey,
        NSURLVolumeIsRemovableKey, NSVolumeEnumerationOptions, NSURL,
    };
    fn flag(url: &NSURL, key: &NSURLResourceKey) -> bool {
        let mut value = None;
        unsafe { url.getResourceValue_forKey_error(&mut value, key) }.is_ok()
            && value
                .and_then(|v| v.downcast::<NSNumber>().ok())
                .is_some_and(|v| v.boolValue())
    }
    let manager = NSFileManager::defaultManager();
    let urls = manager
        .mountedVolumeURLsIncludingResourceValuesForKeys_options(
            None,
            NSVolumeEnumerationOptions::SkipHiddenVolumes,
        )
        .ok_or("无法读取已连接的磁盘")?;
    let mut volumes = vec![volume("/", "文件系统", false)];
    for url in urls.iter() {
        let Some(path) = url.path().map(|p| p.to_string()) else {
            continue;
        };
        if path == "/" {
            continue;
        }
        let name = Path::new(&path)
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.clone());
        let ejectable = unsafe {
            flag(&url, NSURLVolumeIsEjectableKey) || flag(&url, NSURLVolumeIsRemovableKey)
        };
        volumes.push(volume(&path, &name, ejectable));
    }
    Ok(volumes)
}

#[cfg(target_os = "macos")]
fn eject_checked(path: &str) -> Result<(), String> {
    use objc2_app_kit::NSWorkspace;
    use objc2_foundation::{NSString, NSURL};
    objc2::rc::autoreleasepool(|_| {
        let url = NSURL::fileURLWithPath_isDirectory(&NSString::from_str(path), true);
        NSWorkspace::sharedWorkspace()
            .unmountAndEjectDeviceAtURL_error(&url)
            .map_err(|e| format!("无法推出磁盘：{}", e.localizedDescription()))
    })
}

#[cfg(target_os = "linux")]
pub fn list() -> Result<Vec<Volume>, String> {
    let data = std::fs::read_to_string("/proc/self/mountinfo")
        .map_err(|e| format!("无法读取磁盘列表：{e}"))?;
    let mut result = vec![volume("/", "文件系统", false)];
    for line in data.lines() {
        if let Some(encoded) = line.split_whitespace().nth(4) {
            let path = encoded
                .replace("\\040", " ")
                .replace("\\011", "\t")
                .replace("\\012", "\n")
                .replace("\\134", "\\");
            if ["/media/", "/run/media/", "/mnt/"]
                .iter()
                .any(|prefix| path.starts_with(prefix))
                && !result.iter().any(|v| v.path == path)
            {
                let name = Path::new(&path)
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| path.clone());
                result.push(volume(&path, &name, true));
            }
        }
    }
    Ok(result)
}

#[cfg(target_os = "linux")]
fn eject_checked(path: &str) -> Result<(), String> {
    let output = std::process::Command::new("gio")
        .args(["mount", "--unmount", path])
        .output()
        .map_err(|e| format!("无法运行系统卸载工具：{e}"))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "无法推出磁盘：{}",
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

#[cfg(windows)]
pub fn list() -> Result<Vec<Volume>, String> {
    use windows_sys::Win32::{
        Storage::FileSystem::GetDriveTypeW,
        System::WindowsProgramming::{DRIVE_CDROM, DRIVE_REMOVABLE},
    };
    let mut result = Vec::new();
    for letter in b'A'..=b'Z' {
        let path = format!("{}:\\", char::from(letter));
        let wide: Vec<u16> = path.encode_utf16().chain(Some(0)).collect();
        let kind = unsafe { GetDriveTypeW(wide.as_ptr()) };
        if kind == 0 || kind == 1 {
            continue;
        }
        result.push(volume(
            &path,
            &format!("磁盘 ({}:)", char::from(letter)),
            kind == DRIVE_REMOVABLE || kind == DRIVE_CDROM,
        ));
    }
    Ok(result)
}

#[cfg(windows)]
fn eject_checked(path: &str) -> Result<(), String> {
    let drive = path.trim_end_matches('\\');
    let script = "$ErrorActionPreference='Stop'; $v=(New-Object -ComObject Shell.Application).NameSpace(17).ParseName($env:PUDDING_EJECT_DRIVE); if (-not $v) { throw '设备不存在' }; $v.InvokeVerb('Eject')";
    let output = std::process::Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .env("PUDDING_EJECT_DRIVE", drive)
        .output()
        .map_err(|e| format!("无法启动系统推出操作：{e}"))?;
    if !output.status.success() {
        return Err(format!(
            "无法推出磁盘：{}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    for _ in 0..30 {
        if !list()?.iter().any(|v| v.path.eq_ignore_ascii_case(path)) {
            return Ok(());
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    Err("系统尚未确认磁盘已推出，请检查设备是否正在使用。".into())
}

fn can_eject(path: &str, volumes: &[Volume]) -> bool {
    volumes
        .iter()
        .any(|item| item.path == path && item.ejectable)
}

pub fn eject(path: &str) -> Result<(), String> {
    if !can_eject(path, &list()?) {
        return Err("此磁盘不可推出或已断开".into());
    }
    eject_checked(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_exactly_matching_ejectable_volumes_are_allowed() {
        let volumes = vec![
            volume("/", "系统", false),
            volume("/media/usb", "USB", true),
        ];
        assert!(!can_eject("/", &volumes));
        assert!(!can_eject("/media/usb/subfolder", &volumes));
        assert!(!can_eject("/not-a-volume", &volumes));
        assert!(can_eject("/media/usb", &volumes));
    }

    #[cfg(target_os = "macos")]
    #[test]
    #[ignore = "requires access to the macOS volume services"]
    fn lists_macos_volumes_without_ejecting() {
        let volumes = list().unwrap();
        assert!(volumes.iter().any(|v| v.path == "/" && !v.ejectable));
    }
}
