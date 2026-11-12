use serde::Serialize;
use std::{
    fs::{self, OpenOptions},
    io::{self, Read},
    path::{Component, Path, PathBuf},
    time::UNIX_EPOCH,
};

type Result<T> = std::result::Result<T, String>;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileEntry {
    pub(crate) name: String,
    pub(crate) path: String,
    pub(crate) is_dir: bool,
    pub(crate) is_symlink: bool,
    pub(crate) size: u64,
    pub(crate) modified: Option<u64>,
    pub(crate) hidden: bool,
    pub(crate) extension: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectoryListing {
    path: String,
    parent: Option<String>,
    pub(crate) entries: Vec<FileEntry>,
    skipped: usize,
}

#[derive(Serialize)]
pub struct Place {
    id: String,
    name: String,
    path: String,
}

#[derive(Serialize)]
pub struct AppInfo {
    home: String,
    platform: String,
    places: Vec<Place>,
    roots: Vec<Place>,
}

fn display(path: &Path) -> String {
    // Canonical Windows paths may use a verbatim prefix. Keep that prefix internally
    // for long-path support, but present ordinary paths to the webview and user.
    let text = path.to_string_lossy();
    if let Some(unc) = text.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{unc}")
    } else {
        text.strip_prefix(r"\\?\").unwrap_or(&text).to_string()
    }
}

fn io_error(action: &str, path: &Path, error: io::Error) -> String {
    format!("{action}「{}」：{error}", display(path))
}

pub fn resolve_directory(path: &str) -> Result<PathBuf> {
    let expanded = if path == "~" || path.starts_with("~/") || path.starts_with("~\\") {
        dirs::home_dir()
            .ok_or("无法定位主文件夹")?
            .join(path.get(2..).unwrap_or(""))
    } else {
        PathBuf::from(path)
    };
    if !expanded.is_absolute() {
        return Err("请输入绝对路径，或使用 ~ 表示主文件夹".into());
    }
    let canonical = fs::canonicalize(&expanded).map_err(|e| io_error("无法访问", &expanded, e))?;
    if !canonical.is_dir() {
        return Err("此路径不是文件夹".into());
    }
    Ok(canonical)
}

pub fn app_info() -> Result<AppInfo> {
    let home = dirs::home_dir().ok_or("无法定位主文件夹")?;
    let candidates = [
        ("home", "主文件夹", Some(home.clone())),
        ("desktop", "桌面", dirs::desktop_dir()),
        ("documents", "文档", dirs::document_dir()),
        ("downloads", "下载", dirs::download_dir()),
        ("pictures", "图片", dirs::picture_dir()),
        ("music", "音乐", dirs::audio_dir()),
        ("videos", "视频", dirs::video_dir()),
    ];
    let places = candidates
        .into_iter()
        .filter_map(|(id, name, path)| {
            path.filter(|p| p.is_dir()).map(|p| Place {
                id: id.into(),
                name: name.into(),
                path: display(&p),
            })
        })
        .collect();
    let mut roots = Vec::new();
    #[cfg(windows)]
    for letter in b'A'..=b'Z' {
        let path = format!("{}:\\", char::from(letter));
        if Path::new(&path).is_dir() {
            roots.push(Place {
                id: path.clone(),
                name: format!("本地磁盘 ({}:)", char::from(letter)),
                path,
            });
        }
    }
    #[cfg(not(windows))]
    {
        roots.push(Place {
            id: "root".into(),
            name: "文件系统".into(),
            path: "/".into(),
        });
        #[cfg(target_os = "macos")]
        let mounts = vec![PathBuf::from("/Volumes")];
        #[cfg(not(target_os = "macos"))]
        let mounts = vec![PathBuf::from("/media"), PathBuf::from("/mnt")];
        for mount in mounts {
            if let Ok(entries) = fs::read_dir(mount) {
                for entry in entries.flatten().filter(|e| e.path().is_dir()) {
                    roots.push(Place {
                        id: display(&entry.path()),
                        name: entry.file_name().to_string_lossy().into(),
                        path: display(&entry.path()),
                    });
                }
            }
        }
    }
    Ok(AppInfo {
        home: display(&home),
        platform: std::env::consts::OS.into(),
        places,
        roots,
    })
}

pub fn read_directory(path: &str) -> Result<DirectoryListing> {
    let directory = resolve_directory(path)?;
    let reader = fs::read_dir(&directory).map_err(|e| io_error("无法读取", &directory, e))?;
    let mut entries = Vec::new();
    let mut skipped = 0;
    for result in reader {
        let Ok(entry) = result else {
            skipped += 1;
            continue;
        };
        let path = entry.path();
        let Ok(link_metadata) = fs::symlink_metadata(&path) else {
            skipped += 1;
            continue;
        };
        let is_symlink = link_metadata.is_symlink();
        let target_metadata = if is_symlink {
            fs::metadata(&path).ok()
        } else {
            None
        };
        let metadata = target_metadata.as_ref().unwrap_or(&link_metadata);
        let name = entry.file_name().to_string_lossy().to_string();
        let hidden = name.starts_with('.');
        #[cfg(windows)]
        let hidden = {
            use std::os::windows::fs::MetadataExt;
            hidden || link_metadata.file_attributes() & 0x2 != 0
        };
        entries.push(FileEntry {
            hidden,
            name,
            path: display(&path),
            is_dir: metadata.is_dir(),
            is_symlink,
            size: if metadata.is_dir() { 0 } else { metadata.len() },
            modified: metadata
                .modified()
                .ok()
                .and_then(|v| v.duration_since(UNIX_EPOCH).ok())
                .map(|v| v.as_secs()),
            extension: path
                .extension()
                .map(|s| s.to_string_lossy().to_lowercase())
                .unwrap_or_default(),
        });
    }
    Ok(DirectoryListing {
        path: display(&directory),
        parent: directory.parent().map(display),
        entries,
        skipped,
    })
}

pub fn validate_name(name: &str) -> Result<()> {
    let mut components = Path::new(name).components();
    let valid_component =
        matches!(components.next(), Some(Component::Normal(_))) && components.next().is_none();
    let base = name.split('.').next().unwrap_or("").to_uppercase();
    let reserved = ["CON", "PRN", "AUX", "NUL"].contains(&base.as_str())
        || (base.len() == 4
            && (base.starts_with("COM") || base.starts_with("LPT"))
            && matches!(base.as_bytes()[3], b'1'..=b'9'));
    if name.trim().is_empty()
        || !valid_component
        || name.ends_with(['.', ' '])
        || reserved
        || name
            .chars()
            .any(|c| c.is_control() || "<>:\"/\\|?*".contains(c))
    {
        return Err(
            "名称无效：请使用单个名称，避免特殊字符、系统保留名称及末尾的空格或句点".into(),
        );
    }
    Ok(())
}

fn exists(path: &Path) -> bool {
    // Includes dangling symlinks, which must never be overwritten.
    fs::symlink_metadata(path).is_ok()
}

pub fn create_folder(parent: &str, name: &str) -> Result<String> {
    validate_name(name)?;
    let path = resolve_directory(parent)?.join(name);
    fs::create_dir(&path).map_err(|e| io_error("无法创建文件夹", &path, e))?;
    Ok(display(&path))
}

fn checked_source(path: &str) -> Result<PathBuf> {
    let path = PathBuf::from(path);
    if !path.is_absolute() || path.file_name().is_none() {
        return Err("无法操作文件系统根目录或相对路径".into());
    }
    // Canonicalize the parent, preserving the entry itself when it is a symlink.
    let parent = path.parent().ok_or("无法操作此路径")?;
    let source = resolve_directory(&display(parent))?.join(path.file_name().unwrap());
    fs::symlink_metadata(&source).map_err(|e| io_error("无法访问", &source, e))?;
    Ok(source)
}

pub fn rename_entry(path: &str, name: &str) -> Result<String> {
    validate_name(name)?;
    let source = checked_source(path)?;
    let destination = source.with_file_name(name);
    if source == destination {
        return Ok(display(&source));
    }
    if exists(&destination) {
        return Err("已存在同名项目，请使用其他名称".into());
    }
    crate::rename::exclusive(&source, &destination)
        .map_err(|e| io_error("无法重命名", &source, e))?;
    Ok(display(&destination))
}

fn copy_entry(source: &Path, destination: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(source).map_err(|e| io_error("无法读取", source, e))?;
    if metadata.is_symlink() {
        return Err(format!("暂不支持复制符号链接：{}", display(source)));
    }
    if metadata.is_dir() {
        fs::create_dir(destination).map_err(|e| io_error("无法创建", destination, e))?;
        let result = (|| {
            for entry in fs::read_dir(source).map_err(|e| io_error("无法读取", source, e))? {
                let entry = entry.map_err(|e| io_error("无法读取目录项目", source, e))?;
                copy_entry(&entry.path(), &destination.join(entry.file_name()))?;
            }
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_dir_all(destination);
        }
        result
    } else if metadata.is_file() {
        let mut input = fs::File::open(source).map_err(|e| io_error("无法读取", source, e))?;
        let mut output = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(destination)
            .map_err(|e| io_error("无法创建", destination, e))?;
        if let Err(error) = io::copy(&mut input, &mut output) {
            drop(output);
            let _ = fs::remove_file(destination);
            return Err(io_error("无法复制", source, error));
        }
        // Preserve executable bits and read-only attributes where supported.
        if let Err(error) = fs::set_permissions(destination, metadata.permissions()) {
            drop(output);
            let _ = fs::remove_file(destination);
            return Err(io_error("无法设置权限", destination, error));
        }
        Ok(())
    } else {
        Err(format!("暂不支持复制特殊文件：{}", display(source)))
    }
}

pub fn transfer_entries(paths: Vec<String>, destination: &str, mode: &str) -> Result<Vec<String>> {
    if mode != "copy" && mode != "cut" {
        return Err("未知文件操作".into());
    }
    if paths.is_empty() {
        return Err("请选择文件或文件夹".into());
    }
    let destination = resolve_directory(destination)?;
    let mut plan = Vec::new();
    for path in &paths {
        let source = checked_source(path)?;
        let target = destination.join(source.file_name().ok_or("无法操作根目录")?);
        if source == target || exists(&target) || plan.iter().any(|(_, t)| t == &target) {
            return Err(format!("目标中已存在「{}」，操作未开始", display(&target)));
        }
        if destination.starts_with(&source) {
            return Err("不能将文件夹放入其自身或子文件夹".into());
        }
        plan.push((source, target));
    }
    let mut completed = Vec::new();
    for (source, target) in plan {
        let result = if mode == "cut" {
            // A failed cross-device rename leaves the source intact.
            crate::rename::exclusive(&source, &target)
                .map_err(|e| io_error("无法移动（跨磁盘移动请先复制再移入回收站）", &source, e))
        } else {
            copy_entry(&source, &target)
        };
        if let Err(error) = result {
            return Err(format!(
                "{error}。已完成 {} 个项目，请刷新查看；源文件不会因复制失败而删除",
                completed.len()
            ));
        }
        completed.push(display(&target));
    }
    Ok(completed)
}

pub fn trash_entries(paths: Vec<String>) -> Result<()> {
    if paths.is_empty() {
        return Err("请选择要移入回收站的项目".into());
    }
    let checked: Vec<_> = paths
        .iter()
        .map(|p| checked_source(p))
        .collect::<Result<_>>()?;
    trash::delete_all(&checked).map_err(|e| format!("无法移入回收站：{e}。请刷新确认项目状态"))
}

pub fn delete_entries(paths: Vec<String>) -> Result<()> {
    if paths.is_empty() {
        return Err("请选择要永久删除的项目".into());
    }
    let checked: Vec<_> = paths
        .iter()
        .map(|p| checked_source(p))
        .collect::<Result<_>>()?;
    for (index, path) in checked.iter().enumerate() {
        let result = (|| {
            let metadata = fs::symlink_metadata(path)?;
            #[cfg(windows)]
            {
                use std::os::windows::fs::FileTypeExt;
                if metadata.file_type().is_symlink_dir() {
                    return fs::remove_dir(path);
                }
            }
            if metadata.is_dir() && !metadata.is_symlink() {
                fs::remove_dir_all(path)
            } else {
                fs::remove_file(path)
            }
        })();
        if let Err(error) = result {
            return Err(format!(
                "{}。已永久删除 {index} 个项目，请刷新确认项目状态",
                io_error("无法永久删除", path, error)
            ));
        }
    }
    Ok(())
}

pub fn openable_path(path: &str) -> Result<String> {
    let path = checked_source(path)?;
    Ok(display(&path))
}

pub fn video_preview_path(path: &str) -> Result<String> {
    let path = checked_source(path)?;
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if !matches!(extension.as_str(), "mp4" | "mov" | "mkv" | "avi" | "webm") {
        return Err("此文件格式不支持视频封面".into());
    }
    let metadata = fs::metadata(&path).map_err(|e| io_error("无法读取视频", &path, e))?;
    if !metadata.is_file() {
        return Err("只能预览普通视频文件".into());
    }
    let resolved = fs::canonicalize(&path).map_err(|e| io_error("无法访问视频", &path, e))?;
    Ok(display(&resolved))
}

const MAX_IMAGE_BYTES: u64 = 32 * 1024 * 1024;

pub fn read_image(path: &str) -> Result<Vec<u8>> {
    let path = checked_source(path)?;
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if !matches!(
        extension.as_str(),
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" | "avif" | "heic" | "bmp"
    ) {
        return Err("此文件格式不支持图片预览".into());
    }
    let metadata = fs::metadata(&path).map_err(|e| io_error("无法读取图片", &path, e))?;
    if !metadata.is_file() {
        return Err("只能预览普通图片文件".into());
    }
    if metadata.len() > MAX_IMAGE_BYTES {
        return Err("图片超过 32 MB，请使用默认应用打开".into());
    }
    let file = fs::File::open(&path).map_err(|e| io_error("无法读取图片", &path, e))?;
    let mut bytes = Vec::new();
    // Limit the read as well, in case another process grows the file after metadata().
    file.take(MAX_IMAGE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| io_error("无法读取图片", &path, e))?;
    if bytes.len() as u64 > MAX_IMAGE_BYTES {
        return Err("图片超过 32 MB，请使用默认应用打开".into());
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn permanently_deletes_files_and_directories_after_validating_all_paths() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("file");
        let folder = dir.path().join("folder");
        fs::write(&file, "data").unwrap();
        fs::create_dir(&folder).unwrap();
        fs::write(folder.join("nested"), "data").unwrap();
        assert!(
            delete_entries(vec![display(&file), display(&dir.path().join("missing"))]).is_err()
        );
        assert!(file.exists());
        assert!(delete_entries(vec!["/".into()]).is_err());
        delete_entries(vec![display(&file), display(&folder)]).unwrap();
        assert!(!file.exists());
        assert!(!folder.exists());
    }

    #[cfg(unix)]
    #[test]
    fn permanently_deleting_a_symlink_preserves_its_target() {
        let dir = tempdir().unwrap();
        let target = dir.path().join("target");
        let link = dir.path().join("link");
        fs::create_dir(&target).unwrap();
        fs::write(target.join("file"), "data").unwrap();
        std::os::unix::fs::symlink(&target, &link).unwrap();
        delete_entries(vec![display(&link)]).unwrap();
        assert!(target.join("file").exists());
        assert!(fs::symlink_metadata(link).is_err());
    }

    #[test]
    fn video_preview_checks_type_and_resolves_regular_files() {
        let dir = tempdir().unwrap();
        let video = dir.path().join("封面.MP4");
        fs::write(&video, "video").unwrap();
        assert_eq!(
            video_preview_path(&display(&video)).unwrap(),
            display(&fs::canonicalize(&video).unwrap())
        );
        let text = dir.path().join("file.txt");
        fs::write(&text, "not a video").unwrap();
        assert!(video_preview_path(&display(&text)).is_err());
        let folder = dir.path().join("folder.mp4");
        fs::create_dir(&folder).unwrap();
        assert!(video_preview_path(&display(&folder)).is_err());
        assert!(video_preview_path("relative.mp4").is_err());
    }

    #[test]
    fn reads_image_bytes_and_rejects_unsupported_or_missing_files() {
        let dir = tempdir().unwrap();
        let image = dir.path().join("图片 #1.PNG");
        let bytes = b"\x89PNG\r\n\x1a\n";
        fs::write(&image, bytes).unwrap();
        assert_eq!(read_image(&display(&image)).unwrap(), bytes);
        let text = dir.path().join("notes.txt");
        fs::write(&text, "notes").unwrap();
        assert!(read_image(&display(&text)).unwrap_err().contains("格式"));
        assert!(read_image(&display(&dir.path().join("missing.png"))).is_err());
        fs::create_dir(dir.path().join("folder.png")).unwrap();
        assert!(read_image(&display(&dir.path().join("folder.png")))
            .unwrap_err()
            .contains("普通"));
        assert!(read_image("relative.png").is_err());
    }

    #[test]
    fn rejects_oversized_images_before_reading_contents() {
        let dir = tempdir().unwrap();
        let image = dir.path().join("large.png");
        fs::File::create(&image)
            .unwrap()
            .set_len(MAX_IMAGE_BYTES + 1)
            .unwrap();
        assert!(read_image(&display(&image)).unwrap_err().contains("32 MB"));
    }

    #[cfg(unix)]
    #[test]
    fn previews_image_symlinks_and_rejects_special_files() {
        use std::os::unix::fs::symlink;
        let dir = tempdir().unwrap();
        let source = dir.path().join("original.svg");
        let link = dir.path().join("linked.svg");
        fs::write(&source, "<svg/>").unwrap();
        symlink(&source, &link).unwrap();
        assert_eq!(read_image(&display(&link)).unwrap(), b"<svg/>");
        let device = dir.path().join("device.png");
        symlink("/dev/null", &device).unwrap();
        assert!(read_image(&display(&device)).unwrap_err().contains("普通"));
    }

    #[test]
    fn reads_real_entries_and_parent() {
        let dir = tempdir().unwrap();
        fs::create_dir(dir.path().join("folder")).unwrap();
        fs::write(dir.path().join(".hidden"), "hello").unwrap();
        let listing = read_directory(&display(dir.path())).unwrap();
        assert_eq!(listing.entries.len(), 2);
        assert!(listing.parent.is_some());
        let hidden = listing
            .entries
            .iter()
            .find(|e| e.name == ".hidden")
            .unwrap();
        assert!(hidden.hidden);
        assert_eq!(hidden.size, 5);
        assert!(listing
            .entries
            .iter()
            .any(|e| e.name == "folder" && e.is_dir));
        assert!(read_directory(&display(&dir.path().join(".hidden"))).is_err());
    }

    #[test]
    fn rejects_traversal_and_portability_hazards() {
        for name in [
            "",
            " ",
            ".",
            "..",
            "../outside",
            "a/b",
            "a\\b",
            "CON.txt",
            "LPT1",
            "name.",
            "name ",
            "bad\0name",
        ] {
            assert!(validate_name(name).is_err(), "accepted {name:?}");
        }
        assert!(validate_name("设计资料 2026").is_ok());
    }

    #[test]
    fn create_and_rename_never_intentionally_overwrite() {
        let dir = tempdir().unwrap();
        let parent = display(dir.path());
        let path = create_folder(&parent, "one").unwrap();
        assert!(create_folder(&parent, "one").is_err());
        create_folder(&parent, "two").unwrap();
        assert!(rename_entry(&path, "two").is_err());
        let renamed = rename_entry(&path, "three").unwrap();
        assert!(Path::new(&renamed).is_dir());
        assert!(!Path::new(&path).exists());
    }

    #[test]
    fn recursive_copy_checks_conflicts_and_descendants() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source");
        let target = dir.path().join("target");
        fs::create_dir_all(source.join("nested")).unwrap();
        fs::create_dir(&target).unwrap();
        fs::write(source.join("nested/file.txt"), "keep me").unwrap();
        let paths = vec![display(&source)];
        assert!(transfer_entries(paths.clone(), &display(&source.join("nested")), "copy").is_err());
        transfer_entries(paths.clone(), &display(&target), "copy").unwrap();
        assert_eq!(
            fs::read_to_string(target.join("source/nested/file.txt")).unwrap(),
            "keep me"
        );
        assert!(source.join("nested/file.txt").exists());
        assert!(transfer_entries(paths, &display(&target), "copy").is_err());
    }

    #[test]
    fn move_and_invalid_modes() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("file.txt");
        let target = dir.path().join("target");
        fs::write(&source, "content").unwrap();
        fs::create_dir(&target).unwrap();
        assert!(transfer_entries(vec![display(&source)], &display(&target), "invalid").is_err());
        transfer_entries(vec![display(&source)], &display(&target), "cut").unwrap();
        assert!(!source.exists());
        assert_eq!(
            fs::read_to_string(target.join("file.txt")).unwrap(),
            "content"
        );
    }

    #[cfg(unix)]
    #[test]
    fn symlinks_are_listed_and_failed_copy_is_cleaned_up() {
        use std::os::unix::fs::symlink;
        let dir = tempdir().unwrap();
        let source = dir.path().join("source");
        let target = dir.path().join("target");
        fs::create_dir(&source).unwrap();
        fs::create_dir(&target).unwrap();
        symlink("/nonexistent-pudding-target", source.join("broken")).unwrap();
        let listing = read_directory(&display(&source)).unwrap();
        assert!(listing.entries[0].is_symlink);
        assert!(transfer_entries(vec![display(&source)], &display(&target), "copy").is_err());
        assert!(!target.join("source").exists());
        assert!(fs::symlink_metadata(source.join("broken")).is_ok());
    }
}
