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
    pub(crate) metadata_pending: bool,
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
    roots: Vec<crate::volumes::Volume>,
}

pub(crate) fn display(path: &Path) -> String {
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

// Count file contents only, without following nested symbolic links.
pub fn directory_size(path: &str) -> Result<u64> {
    let root = resolve_directory(path)?;
    let mut pending = vec![root];
    let mut total = 0u64;
    while let Some(directory) = pending.pop() {
        for item in fs::read_dir(&directory).map_err(|e| io_error("无法读取", &directory, e))? {
            let item = item.map_err(|e| io_error("无法读取", &directory, e))?;
            let path = item.path();
            let metadata =
                fs::symlink_metadata(&path).map_err(|e| io_error("无法读取", &path, e))?;
            if metadata.is_symlink() {
                continue;
            }
            if metadata.is_dir() {
                pending.push(path);
            } else if metadata.is_file() {
                total = total
                    .checked_add(metadata.len())
                    .ok_or("文件夹大小超出计算范围")?;
            }
        }
    }
    Ok(total)
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
    let roots = crate::volumes::list().unwrap_or_else(|_| crate::volumes::fallback());
    Ok(AppInfo {
        home: display(&home),
        platform: std::env::consts::OS.into(),
        places,
        roots,
    })
}

pub fn read_directory(path: &str) -> Result<DirectoryListing> {
    read_directory_impl(path, true)
}

pub fn read_directory_full(path: &str) -> Result<DirectoryListing> {
    read_directory_impl(path, false)
}

#[cfg(target_os = "macos")]
fn is_smb_directory(path: &Path) -> bool {
    use std::{
        ffi::{CStr, CString},
        os::unix::ffi::OsStrExt,
    };
    let Ok(path) = CString::new(path.as_os_str().as_bytes()) else {
        return false;
    };
    let mut stat = std::mem::MaybeUninit::<libc::statfs>::uninit();
    // SAFETY: statfs writes the entire statfs structure on success.
    if unsafe { libc::statfs(path.as_ptr(), stat.as_mut_ptr()) } != 0 {
        return false;
    }
    let stat = unsafe { stat.assume_init() };
    // SAFETY: macOS provides f_fstypename as a NUL-terminated array.
    unsafe { CStr::from_ptr(stat.f_fstypename.as_ptr()) }.to_bytes() == b"smbfs"
}

#[cfg(not(target_os = "macos"))]
fn is_smb_directory(_path: &Path) -> bool {
    false
}

fn read_directory_impl(path: &str, allow_lazy: bool) -> Result<DirectoryListing> {
    let directory = resolve_directory(path)?;
    let reader = fs::read_dir(&directory).map_err(|e| io_error("无法读取", &directory, e))?;
    // Enumerate first, then bound concurrent metadata requests. On network
    // mounts a serial stat per entry adds one round trip to every file.
    let mut skipped = 0;
    let mut seen = std::collections::HashSet::new();
    let items: Vec<_> = reader
        .filter_map(|item| match item {
            Ok(entry) if seen.insert(entry.file_name()) => Some(entry),
            Ok(_) => None,
            Err(_) => {
                skipped += 1;
                None
            }
        })
        .collect();
    let lazy = allow_lazy && items.len() >= 128 && is_smb_directory(&directory);
    let workers = if items.len() >= 64 { 8 } else { 1 };
    let chunk_size = items.len().div_ceil(workers).max(1);
    let groups = std::thread::scope(|scope| {
        let handles: Vec<_> = items
            .chunks(chunk_size)
            .map(|chunk| {
                scope.spawn(move || {
                    chunk
                        .iter()
                        .map(|entry| {
                            if lazy {
                                directory_entry_basic(entry)
                            } else {
                                directory_entry(entry)
                            }
                        })
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|handle| handle.join().expect("directory metadata worker panicked"))
            .collect::<Vec<_>>()
    });
    let mut entries = Vec::with_capacity(items.len());
    for group in groups {
        for entry in group {
            match entry {
                Some(entry) => entries.push(entry),
                None => skipped += 1,
            }
        }
    }
    Ok(DirectoryListing {
        path: display(&directory),
        parent: directory.parent().map(display),
        entries,
        skipped,
    })
}

fn directory_entry(entry: &fs::DirEntry) -> Option<FileEntry> {
    let path = entry.path();
    // DirEntry::metadata can reuse the directory enumeration's attributes on
    // Windows and, like symlink_metadata, does not follow symbolic links.
    let link_metadata = entry.metadata().ok()?;
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
    Some(FileEntry {
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
        metadata_pending: false,
        extension: path
            .extension()
            .map(|s| s.to_string_lossy().to_lowercase())
            .unwrap_or_default(),
    })
}

fn directory_entry_basic(entry: &fs::DirEntry) -> Option<FileEntry> {
    let kind = entry.file_type().ok()?;
    if kind.is_symlink() {
        return directory_entry(entry);
    }
    let path = entry.path();
    let name = entry.file_name().to_string_lossy().to_string();
    Some(FileEntry {
        hidden: name.starts_with('.'),
        name,
        path: display(&path),
        is_dir: kind.is_dir(),
        is_symlink: false,
        size: 0,
        modified: None,
        metadata_pending: true,
        extension: path
            .extension()
            .map(|s| s.to_string_lossy().to_lowercase())
            .unwrap_or_default(),
    })
}

pub fn read_entry_details(directory: &str, names: Vec<String>) -> Result<Vec<FileEntry>> {
    if names.len() > 64 {
        return Err("一次读取的项目过多".into());
    }
    let directory = resolve_directory(directory)?;
    let mut paths = Vec::with_capacity(names.len());
    for name in names {
        let mut parts = Path::new(&name).components();
        if !matches!(parts.next(), Some(Component::Normal(_))) || parts.next().is_some() {
            return Err("无效的项目名称".into());
        }
        paths.push(directory.join(name));
    }
    let chunk_size = paths.len().div_ceil(4).max(1);
    Ok(std::thread::scope(|scope| {
        let handles: Vec<_> = paths
            .chunks(chunk_size)
            .map(|chunk| {
                scope.spawn(move || {
                    chunk
                        .iter()
                        .filter_map(|path| {
                            let metadata = fs::symlink_metadata(path).ok()?;
                            let target = if metadata.is_symlink() {
                                fs::metadata(path).ok()
                            } else {
                                None
                            };
                            let effective = target.as_ref().unwrap_or(&metadata);
                            let name = path.file_name()?.to_string_lossy().into_owned();
                            Some(FileEntry {
                                hidden: name.starts_with('.'),
                                name,
                                path: display(path),
                                is_dir: effective.is_dir(),
                                is_symlink: metadata.is_symlink(),
                                size: if effective.is_dir() {
                                    0
                                } else {
                                    effective.len()
                                },
                                modified: effective
                                    .modified()
                                    .ok()
                                    .and_then(|v| v.duration_since(UNIX_EPOCH).ok())
                                    .map(|v| v.as_secs()),
                                metadata_pending: false,
                                extension: path
                                    .extension()
                                    .map(|s| s.to_string_lossy().to_lowercase())
                                    .unwrap_or_default(),
                            })
                        })
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        handles
            .into_iter()
            .flat_map(|h| h.join().expect("directory metadata worker panicked"))
            .collect()
    }))
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

pub fn create_file(parent: &str, name: &str) -> Result<String> {
    validate_name(name)?;
    let path = resolve_directory(parent)?.join(name);
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|error| {
            if error.kind() == io::ErrorKind::AlreadyExists {
                "已存在同名项目，请使用其他名称".to_string()
            } else {
                io_error("无法创建文件", &path, error)
            }
        })?;
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

fn transfer_sources(paths: &[String], destination: &str) -> Result<(PathBuf, Vec<PathBuf>)> {
    if paths.is_empty() {
        return Err("请选择文件或文件夹".into());
    }
    let destination = resolve_directory(destination)?;
    let mut sources = Vec::new();
    for path in paths {
        let source = checked_source(path)?;
        if destination.starts_with(&source) {
            return Err("不能将文件夹放入其自身或子文件夹".into());
        }
        if sources.contains(&source) {
            return Err("不能重复操作同一项目".into());
        }
        sources.push(source);
    }
    Ok((destination, sources))
}

pub fn transfer_conflicts(paths: Vec<String>, destination: &str) -> Result<Vec<String>> {
    let (destination, sources) = transfer_sources(&paths, destination)?;
    let mut targets = Vec::new();
    let mut conflicts = Vec::new();
    for source in sources {
        let target = destination.join(source.file_name().ok_or("无法操作根目录")?);
        if exists(&target) || targets.contains(&target) {
            conflicts.push(display(&target));
        }
        targets.push(target);
    }
    Ok(conflicts)
}

fn unique_target(target: &Path, reserved: &[PathBuf]) -> PathBuf {
    let name = target.file_name().unwrap_or_default().to_string_lossy();
    let is_dir = fs::symlink_metadata(target).is_ok_and(|m| m.is_dir());
    let (stem, extension) = if !is_dir && !name.starts_with('.') {
        name.rsplit_once('.')
            .map_or((name.as_ref(), ""), |(stem, ext)| {
                (stem, &name[stem.len()..stem.len() + ext.len() + 1])
            })
    } else {
        (name.as_ref(), "")
    };
    for index in 1u64.. {
        let candidate = target.with_file_name(format!(
            "{stem} 副本{}{extension}",
            if index == 1 {
                String::new()
            } else {
                format!(" {index}")
            }
        ));
        if !exists(&candidate) && !reserved.contains(&candidate) {
            return candidate;
        }
    }
    unreachable!()
}

fn remove_entry(path: &Path) -> std::io::Result<()> {
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
}

fn transfer_one(source: &Path, target: &Path, mode: &str) -> Result<()> {
    if mode == "cut" {
        crate::rename::exclusive(source, target)
            .map_err(|e| io_error("无法移动（跨磁盘移动请先复制再移入回收站）", source, e))
    } else {
        copy_entry(source, target)
    }
}

pub fn transfer_entries_with_policy(
    paths: Vec<String>,
    destination: &str,
    mode: &str,
    policy: &str,
    confirmed: Vec<String>,
) -> Result<Vec<String>> {
    if mode != "copy" && mode != "cut" {
        return Err("未知文件操作".into());
    }
    if !["reject", "replace", "keep"].contains(&policy) {
        return Err("未知冲突处理方式".into());
    }
    let (destination, sources) = transfer_sources(&paths, destination)?;
    let mut plan = Vec::new();
    let mut reserved = Vec::new();
    let all_sources = sources.clone();
    for source in sources {
        let mut target = destination.join(source.file_name().ok_or("无法操作根目录")?);
        let conflict = exists(&target) || reserved.contains(&target);
        if conflict {
            match policy {
                "keep" => target = unique_target(&target, &reserved),
                "replace"
                    if !all_sources.contains(&target)
                        && !reserved.contains(&target)
                        && confirmed.contains(&display(&target)) => {}
                _ => return Err(format!("目标中已存在「{}」，操作未开始", display(&target))),
            }
        }
        reserved.push(target.clone());
        plan.push((source, target, conflict && policy == "replace"));
    }
    let mut completed = Vec::new();
    for (source, target, replace) in plan {
        // Move the existing entry aside first, so a failed operation can restore it.
        let backup = if replace && exists(&target) {
            let backup = unique_target(
                &target.with_file_name(format!(".pudding-backup-{}", std::process::id())),
                &reserved,
            );
            if let Err(e) = crate::rename::exclusive(&target, &backup) {
                return Err(format!(
                    "{}。已完成 {} 个项目",
                    io_error("无法暂存目标", &target, e),
                    completed.len()
                ));
            }
            Some(backup)
        } else {
            None
        };
        if let Err(error) = transfer_one(&source, &target, mode) {
            let restore = backup
                .as_ref()
                .and_then(|backup| {
                    crate::rename::exclusive(backup, &target)
                        .err()
                        .map(|e| format!("；原目标保存在「{}」，恢复失败：{e}", display(backup)))
                })
                .unwrap_or_default();
            return Err(format!(
                "{error}。已完成 {} 个项目，请刷新查看{restore}",
                completed.len()
            ));
        }
        if let Some(backup) = backup {
            if let Err(e) = remove_entry(&backup) {
                return Err(format!(
                    "已完成替换，但无法清理原目标「{}」：{e}。请刷新查看",
                    display(&backup)
                ));
            }
        }
        completed.push(display(&target));
    }
    Ok(completed)
}

#[cfg(test)]
fn transfer_entries(paths: Vec<String>, destination: &str, mode: &str) -> Result<Vec<String>> {
    transfer_entries_with_policy(paths, destination, mode, "reject", vec![])
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
    if !matches!(
        extension.as_str(),
        "mp4" | "m4v" | "mov" | "mkv" | "avi" | "webm" | "mpeg" | "mpg" | "ogv"
    ) {
        return Err("此文件格式不支持视频封面".into());
    }
    let metadata = fs::metadata(&path).map_err(|e| io_error("无法读取视频", &path, e))?;
    if !metadata.is_file() {
        return Err("只能预览普通视频文件".into());
    }
    let resolved = fs::canonicalize(&path).map_err(|e| io_error("无法访问视频", &path, e))?;
    Ok(display(&resolved))
}

pub fn read_preview_file(path: &str, kind: &str) -> Result<Vec<u8>> {
    let path = checked_source(path)?;
    let limit = match kind {
        "text" => 2 * 1024 * 1024,
        "pdf" => {
            if !path
                .extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| e.eq_ignore_ascii_case("pdf"))
            {
                return Err("此文件不是 PDF".into());
            }
            64 * 1024 * 1024
        }
        _ => return Err("不支持的预览类型".into()),
    };
    let metadata = fs::metadata(&path).map_err(|e| io_error("无法读取文件", &path, e))?;
    if !metadata.is_file() {
        return Err("只能预览普通文件".into());
    }
    if metadata.len() > limit {
        return Err(format!(
            "文件超过预览大小限制（{} MB），请使用默认应用打开",
            limit / 1024 / 1024
        ));
    }
    let file = fs::File::open(&path).map_err(|e| io_error("无法读取文件", &path, e))?;
    let mut bytes = Vec::new();
    file.take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| io_error("无法读取文件", &path, e))?;
    if bytes.len() as u64 > limit {
        return Err("文件超过预览大小限制，请使用默认应用打开".into());
    }
    Ok(bytes)
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
    fn preview_reads_regular_files_with_size_and_type_limits() {
        let dir = tempdir().unwrap();
        let text = dir.path().join("文本.txt");
        fs::write(&text, "你好").unwrap();
        assert_eq!(
            read_preview_file(&display(&text), "text").unwrap(),
            "你好".as_bytes()
        );
        assert!(read_preview_file(&display(&text), "pdf").is_err());
        assert!(read_preview_file(&display(dir.path()), "text").is_err());
        assert!(read_preview_file("relative", "text").is_err());
        let pdf = dir.path().join("sample.PDF");
        fs::write(&pdf, b"%PDF").unwrap();
        assert_eq!(read_preview_file(&display(&pdf), "pdf").unwrap(), b"%PDF");
        fs::File::create(&text)
            .unwrap()
            .set_len(2 * 1024 * 1024 + 1)
            .unwrap();
        assert!(read_preview_file(&display(&text), "text")
            .unwrap_err()
            .contains("限制"));
        assert!(read_preview_file(&display(&pdf), "unknown").is_err());
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

    #[cfg(target_os = "macos")]
    #[test]
    #[ignore = "requires PUDDING_TEST_SMB_DIRECTORY to point to an accessible SMB directory"]
    fn reads_smb_directory_without_waiting_for_all_metadata() {
        let path =
            std::env::var("PUDDING_TEST_SMB_DIRECTORY").expect("set PUDDING_TEST_SMB_DIRECTORY");
        let start = std::time::Instant::now();
        let listing = read_directory(&path).unwrap();
        eprintln!(
            "SMB entries: {}, pending: {}, elapsed: {:?}",
            listing.entries.len(),
            listing
                .entries
                .iter()
                .filter(|entry| entry.metadata_pending)
                .count(),
            start.elapsed()
        );
        assert!(
            listing.entries.len() >= 128
                && listing.entries.iter().any(|entry| entry.metadata_pending)
        );
        let names: std::collections::HashSet<_> =
            listing.entries.iter().map(|entry| &entry.name).collect();
        assert_eq!(names.len(), listing.entries.len());
        let sample: Vec<_> = listing
            .entries
            .iter()
            .filter(|entry| entry.metadata_pending)
            .take(8)
            .map(|entry| entry.name.clone())
            .collect();
        let details = read_entry_details(&listing.path, sample).unwrap();
        assert!(details.iter().all(|entry| !entry.metadata_pending));
    }

    #[test]
    fn large_directory_preserves_entries_and_metadata() {
        let dir = tempdir().unwrap();
        for index in 0..160 {
            fs::write(dir.path().join(format!("item-{index:03}.txt")), b"test").unwrap();
        }
        fs::create_dir(dir.path().join("nested")).unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink("missing", dir.path().join("broken-link")).unwrap();
        let listing = read_directory(&display(dir.path())).unwrap();
        assert_eq!(listing.entries.len(), if cfg!(unix) { 162 } else { 161 });
        assert_eq!(listing.skipped, 0);
        assert!(listing
            .entries
            .iter()
            .any(|entry| entry.name == "item-070.txt" && entry.size == 4 && !entry.is_dir));
        assert!(listing
            .entries
            .iter()
            .any(|entry| entry.name == "nested" && entry.is_dir));
        #[cfg(unix)]
        assert!(listing
            .entries
            .iter()
            .any(|entry| entry.name == "broken-link" && entry.is_symlink));
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
    fn create_file_is_empty_validated_and_never_overwrites() {
        let dir = tempdir().unwrap();
        let parent = display(dir.path());
        let path = create_file(&parent, "新建 文件.txt").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"");
        fs::write(&path, b"keep").unwrap();
        assert!(create_file(&parent, "新建 文件.txt")
            .unwrap_err()
            .contains("同名"));
        assert_eq!(fs::read(&path).unwrap(), b"keep");
        fs::create_dir(dir.path().join("folder")).unwrap();
        assert!(create_file(&parent, "folder").is_err());
        assert!(create_file(&parent, "../outside").is_err());
        assert!(create_file(&parent, "CON.txt").is_err());
        assert!(create_file(&parent, "no-extension").is_ok());
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink("missing", dir.path().join("broken")).unwrap();
            assert!(create_file(&parent, "broken").is_err());
            assert!(fs::symlink_metadata(dir.path().join("broken"))
                .unwrap()
                .is_symlink());
        }
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
    fn conflicts_can_keep_both_or_replace_files_and_directories() {
        let root = tempdir().unwrap();
        let source = root.path().join("source");
        let dest = root.path().join("dest");
        fs::create_dir(&source).unwrap();
        fs::create_dir(&dest).unwrap();
        let source = source.canonicalize().unwrap();
        let dest = dest.canonicalize().unwrap();
        fs::write(source.join("报告.txt"), "new").unwrap();
        fs::write(dest.join("报告.txt"), "old").unwrap();
        fs::write(dest.join("报告 副本.txt"), "other").unwrap();
        let paths = vec![display(&source.join("报告.txt"))];
        let conflicts = transfer_conflicts(paths.clone(), &display(&dest)).unwrap();
        assert_eq!(conflicts, vec![display(&dest.join("报告.txt"))]);
        let copied = transfer_entries_with_policy(
            paths.clone(),
            &display(&dest),
            "copy",
            "keep",
            conflicts.clone(),
        )
        .unwrap();
        assert_eq!(copied, vec![display(&dest.join("报告 副本 2.txt"))]);
        assert_eq!(fs::read_to_string(dest.join("报告.txt")).unwrap(), "old");
        assert!(transfer_entries_with_policy(
            paths.clone(),
            &display(&dest),
            "copy",
            "replace",
            vec![]
        )
        .is_err());
        transfer_entries_with_policy(paths, &display(&dest), "cut", "replace", conflicts).unwrap();
        assert_eq!(fs::read_to_string(dest.join("报告.txt")).unwrap(), "new");
        assert!(!source.join("报告.txt").exists());
        fs::create_dir(source.join("folder")).unwrap();
        fs::write(source.join("folder/new"), "new").unwrap();
        fs::create_dir(dest.join("folder")).unwrap();
        fs::write(dest.join("folder/old"), "old").unwrap();
        let paths = vec![display(&source.join("folder"))];
        transfer_entries_with_policy(
            paths,
            &display(&dest),
            "copy",
            "replace",
            vec![display(&dest.join("folder"))],
        )
        .unwrap();
        assert!(dest.join("folder/new").exists());
        assert!(!dest.join("folder/old").exists());
    }

    #[cfg(unix)]
    #[test]
    fn failed_replacement_restores_original_and_same_folder_can_keep_both() {
        use std::os::unix::fs::symlink;
        let root = tempdir().unwrap();
        let source = root.path().join("source");
        let dest = root.path().join("dest");
        fs::create_dir(&source).unwrap();
        fs::create_dir(&dest).unwrap();
        let source = source.canonicalize().unwrap();
        let dest = dest.canonicalize().unwrap();
        symlink("missing", source.join("link")).unwrap();
        fs::write(dest.join("link"), "original").unwrap();
        let paths = vec![display(&source.join("link"))];
        assert!(transfer_entries_with_policy(
            paths,
            &display(&dest),
            "copy",
            "replace",
            vec![display(&dest.join("link"))]
        )
        .is_err());
        assert_eq!(fs::read_to_string(dest.join("link")).unwrap(), "original");
        assert_eq!(fs::read_dir(&dest).unwrap().count(), 1);
        fs::write(source.join("file.txt"), "content").unwrap();
        let paths = vec![display(&source.join("file.txt"))];
        assert!(transfer_entries_with_policy(
            paths.clone(),
            &display(&source),
            "copy",
            "replace",
            vec![display(&source.join("file.txt"))]
        )
        .is_err());
        transfer_entries_with_policy(paths, &display(&source), "copy", "keep", vec![]).unwrap();
        assert_eq!(
            fs::read_to_string(source.join("file 副本.txt")).unwrap(),
            "content"
        );
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

#[cfg(test)]
mod directory_size_tests {
    use super::*;

    #[test]
    fn counts_nested_files_without_following_links() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join("nested")).unwrap();
        fs::write(root.path().join("a"), b"123").unwrap();
        fs::write(root.path().join("nested/b"), b"12345").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(root.path(), root.path().join("nested/link")).unwrap();
        assert_eq!(directory_size(root.path().to_str().unwrap()).unwrap(), 8);
    }
}
