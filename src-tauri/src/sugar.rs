use std::{path::PathBuf, process::Command};

fn supports_protocol(path: &std::path::Path) -> bool {
    Command::new(path)
        .arg("--protocol-version")
        .output()
        .is_ok_and(|result| result.status.success() && result.stdout == b"sugar-pudding-2\n")
}

fn helper() -> Option<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(path) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path) {
            candidates.push(dir.join(if cfg!(windows) {
                "sugar-archive-pudding.exe"
            } else {
                "sugar-archive-pudding"
            }));
        }
    }
    #[cfg(target_os = "macos")]
    {
        if let Some(path) = objc2::rc::autoreleasepool(|_| {
            use objc2_app_kit::NSWorkspace;
            use objc2_foundation::NSString;
            NSWorkspace::sharedWorkspace()
                .URLForApplicationWithBundleIdentifier(&NSString::from_str("com.sugararchive.app"))
                .and_then(|url| url.path())
                .map(|path| PathBuf::from(path.to_string()))
        }) {
            candidates.push(path.join("Contents/MacOS/sugar-archive-pudding"));
        }
        candidates.push(PathBuf::from(
            "/Applications/Sugar Archive.app/Contents/MacOS/sugar-archive-pudding",
        ));
        if let Some(home) = dirs::home_dir() {
            candidates.push(
                home.join("Applications/Sugar Archive.app/Contents/MacOS/sugar-archive-pudding"),
            );
        }
    }
    find_helper(candidates)
}

fn find_helper(candidates: impl IntoIterator<Item = PathBuf>) -> Option<PathBuf> {
    candidates
        .into_iter()
        .find(|path| path.is_file() && supports_protocol(path))
}

pub fn available() -> bool {
    helper().is_some()
}

// macOS current_exe() preserves the helper symlink, so Sugar's stage command
// would relaunch the CLI instead of the GUI. Launch the real executable here.
#[cfg(target_os = "macos")]
fn macos_custom_command(helper: &std::path::Path, paths: &[PathBuf]) -> Option<Command> {
    use std::process::Stdio;
    let executable = std::fs::canonicalize(helper).ok()?;
    if executable.file_name()? != "sugar-archive" {
        return None;
    }
    let mut command = Command::new(executable);
    command
        .arg("--pudding-stage")
        .args(paths)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    Some(command)
}

fn archive_name(name: &str) -> Option<&str> {
    let lower = name.to_ascii_lowercase();
    [
        ".tar.bz2", ".tar.gz", ".tar.xz", ".tbz2", ".tgz", ".tbz", ".txz", ".zip", ".tar", ".7z",
    ]
    .iter()
    .find(|suffix| lower.ends_with(**suffix))
    .and_then(|suffix| name.get(..name.len() - suffix.len()))
    .filter(|name| !name.is_empty())
}

pub fn run(action: &str, paths: Vec<String>) -> Result<(), String> {
    if paths.is_empty() {
        return Err("请选择文件或文件夹".into());
    }
    let helper = helper().ok_or("未找到支持联动的 Sugar，请安装新版 Sugar")?;
    let paths: Vec<PathBuf> = paths
        .iter()
        .map(|p| crate::filesystem::openable_path(p).map(PathBuf::from))
        .collect::<Result<_, _>>()?;
    #[cfg(target_os = "macos")]
    if action == "custom" {
        if let Some(mut command) = macos_custom_command(&helper, &paths) {
            return command
                .spawn()
                .map(|_| ())
                .map_err(|error| format!("无法启动 Sugar：{error}"));
        }
    }
    let mut command = Command::new(helper);
    let mut created_dir = None;
    match action {
        "extract" | "extract-folder" => {
            if paths.len() != 1 || !paths[0].is_file() {
                return Err("请选择一个压缩包".into());
            }
            let archive = &paths[0];
            let name = archive
                .file_name()
                .and_then(|s| s.to_str())
                .and_then(archive_name)
                .ok_or("不支持此压缩包格式")?;
            let parent = archive.parent().ok_or("找不到压缩包所在文件夹")?;
            let dest = if action == "extract-folder" {
                let dir = parent.join(name);
                std::fs::create_dir(&dir).map_err(|e| format!("无法创建解压文件夹：{e}"))?;
                created_dir = Some(dir.clone());
                dir
            } else {
                parent.to_path_buf()
            };
            command.arg("extract").arg(archive).arg(dest);
        }
        "custom" => {
            command.arg("stage").args(&paths);
        }
        "compress" => {
            let first = &paths[0];
            let parent = first.parent().ok_or("找不到文件所在文件夹")?;
            if paths.iter().any(|path| path.parent() != Some(parent)) {
                return Err("请选择同一文件夹内的项目".into());
            }
            let name = if paths.len() == 1 {
                first
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned()
            } else {
                "压缩文件".into()
            };
            let output = parent.join(format!("{name}.zip"));
            command.arg("compress").arg(output).args(&paths);
        }
        _ => return Err("未知的压缩操作".into()),
    }
    let result = command.output();
    match result {
        Ok(output) if output.status.success() => Ok(()),
        result => {
            if let Some(dir) = created_dir {
                let _ = std::fs::remove_dir(dir);
            }
            match result {
                Ok(output) => Err(format!(
                    "Sugar 操作失败：{}",
                    String::from_utf8_lossy(&output.stderr).trim()
                )),
                Err(error) => Err(format!("无法启动 Sugar：{error}")),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(unix)]
    #[test]
    fn skips_incompatible_helper_before_valid_candidate() {
        use std::os::unix::fs::PermissionsExt;
        let temp = tempfile::tempdir().unwrap();
        let invalid = temp.path().join("old-helper");
        let valid = temp.path().join("new-helper");
        for (path, version) in [(&invalid, "old"), (&valid, "sugar-pudding-2")] {
            std::fs::write(path, format!("#!/bin/sh\nprintf '%s\\n' '{version}'\n")).unwrap();
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        assert_eq!(find_helper([invalid, valid.clone()]), Some(valid));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn custom_compression_launches_real_executable_with_sources() {
        let temp = tempfile::tempdir().unwrap();
        let executable = temp.path().join("sugar-archive");
        let helper = temp.path().join("sugar-archive-pudding");
        std::fs::write(&executable, "").unwrap();
        std::os::unix::fs::symlink(&executable, &helper).unwrap();
        let source = temp.path().join("中文 file.txt");
        let command = macos_custom_command(&helper, std::slice::from_ref(&source)).unwrap();
        assert_eq!(
            command.get_program(),
            executable.canonicalize().unwrap().as_os_str()
        );
        assert_eq!(
            command.get_args().collect::<Vec<_>>(),
            [std::ffi::OsStr::new("--pudding-stage"), source.as_os_str()]
        );
    }

    #[test]
    fn supported_archive_suffixes() {
        assert_eq!(archive_name("报告.TAR.GZ"), Some("报告"));
        assert_eq!(archive_name("file.zip"), Some("file"));
        assert_eq!(archive_name("file.gz"), None);
        assert_eq!(archive_name(".zip"), None);
    }
}
