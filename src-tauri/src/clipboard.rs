#[cfg(not(target_os = "macos"))]
use clipboard_rs::ClipboardContent;
use clipboard_rs::{Clipboard, ClipboardContext, ContentFormat};
use serde::{Deserialize, Serialize};
use std::{path::Path, sync::Mutex};

const MODE: &str = "app.pudding.file-operation";
const GNOME: &str = "x-special/gnome-copied-files";
const EFFECT: &str = "Preferred DropEffect";

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct FileClipboard {
    pub paths: Vec<String>,
    pub mode: String,
}

// Keep the context alive: on X11 the owning process serves clipboard requests.
#[derive(Default)]
pub struct SystemClipboard(Mutex<Option<ClipboardContext>>);

fn local_paths(files: Vec<String>) -> Result<Vec<String>, String> {
    let mut paths = Vec::new();
    for file in files {
        let path = if file.starts_with("file:") {
            url::Url::parse(&file)
                .map_err(|_| "无效的文件 URL")?
                .to_file_path()
                .map_err(|_| "剪贴板仅支持本地文件")?
        } else {
            file.into()
        };
        if !path.is_absolute() {
            return Err("剪贴板包含非绝对路径".into());
        }
        // Directory file URLs may end in '/'. Normalize redundant separators
        // without resolving symlinks or changing '..' semantics.
        let path = path
            .components()
            .collect::<std::path::PathBuf>()
            .to_string_lossy()
            .into_owned();
        if !paths.contains(&path) {
            paths.push(path);
        }
    }
    Ok(paths)
}

fn read(context: &ClipboardContext) -> Result<Option<FileClipboard>, String> {
    let formats = context.available_formats().map_err(|e| e.to_string())?;
    let gnome = if formats.iter().any(|f| f == GNOME) {
        context.get_buffer(GNOME).ok()
    } else {
        None
    };
    let files = if context.has(ContentFormat::Files) {
        context.get_files().map_err(|e| e.to_string())?
    } else if let Some(data) = &gnome {
        String::from_utf8_lossy(data)
            .lines()
            .skip(1)
            .filter(|line| !line.is_empty())
            .map(str::to_owned)
            .collect()
    } else {
        return Ok(None);
    };
    let paths = local_paths(files)?;
    if paths.is_empty() {
        return Ok(None);
    }
    let buffer = |name: &str| {
        formats
            .iter()
            .any(|f| f == name)
            .then(|| context.get_buffer(name).ok())
            .flatten()
    };
    let cut = buffer(MODE).as_deref() == Some(b"cut")
        || buffer(EFFECT).as_deref() == Some(&2u32.to_le_bytes())
        || gnome
            .as_deref()
            .is_some_and(|data| data.starts_with(b"cut\n"));
    Ok(Some(FileClipboard {
        paths,
        mode: if cut { "cut" } else { "copy" }.into(),
    }))
}

impl SystemClipboard {
    fn with<T>(
        &self,
        work: impl FnOnce(&ClipboardContext) -> Result<T, String>,
    ) -> Result<T, String> {
        let mut context = self.0.lock().map_err(|_| "剪贴板正在使用中")?;
        if context.is_none() {
            *context =
                Some(ClipboardContext::new().map_err(|e| format!("无法访问系统剪贴板：{e}"))?);
        }
        work(context.as_ref().unwrap())
    }

    pub fn read(&self) -> Result<Option<FileClipboard>, String> {
        self.with(read)
    }

    pub fn write(&self, operation: FileClipboard) -> Result<(), String> {
        if operation.mode != "copy" && operation.mode != "cut" {
            return Err("未知剪贴操作".into());
        }
        let paths = local_paths(operation.paths)?;
        if paths.is_empty() {
            return Err("请选择文件或文件夹".into());
        }
        for path in &paths {
            std::fs::symlink_metadata(path).map_err(|e| format!("无法复制「{path}」：{e}"))?;
        }
        #[cfg(target_os = "macos")]
        return self.with(|_| {
            write_macos(
                &objc2_app_kit::NSPasteboard::generalPasteboard(),
                &paths,
                &operation.mode,
            )
        });
        #[cfg(not(target_os = "macos"))]
        self.with(|context| {
            let mut contents = vec![ClipboardContent::Other(
                MODE.into(),
                operation.mode.as_bytes().to_vec(),
            )];
            #[cfg(not(target_os = "linux"))]
            contents.push(ClipboardContent::Files(paths.clone()));
            #[cfg(target_os = "windows")]
            contents.push(ClipboardContent::Other(
                EFFECT.into(),
                if operation.mode == "cut" { 2u32 } else { 1u32 }
                    .to_le_bytes()
                    .to_vec(),
            ));
            #[cfg(target_os = "linux")]
            {
                let uris: Vec<String> = paths
                    .iter()
                    .map(|p| url::Url::from_file_path(p).unwrap().into())
                    .collect();
                // Avoid the library's Files variant here: it unconditionally
                // adds a second GNOME representation with a copy action.
                contents.push(ClipboardContent::Other(
                    "text/uri-list".into(),
                    uris.join("\r\n").into_bytes(),
                ));
                contents.push(ClipboardContent::Other(
                    GNOME.into(),
                    format!("{}\n{}", operation.mode, uris.join("\n")).into_bytes(),
                ));
            }
            // A single multi-format write keeps the file list and operation together.
            context
                .set(contents)
                .map_err(|e| format!("写入系统剪贴板失败：{e}"))
        })
    }

    pub fn clear(&self, expected: FileClipboard) -> Result<(), String> {
        self.with(|context| {
            if read(context)?.as_ref() == Some(&expected) {
                context.clear().map_err(|e| e.to_string())?;
            }
            Ok(())
        })
    }

    pub fn text(&self, text: String) -> Result<(), String> {
        self.with(|context| context.set_text(text).map_err(|e| e.to_string()))
    }
}

#[cfg(target_os = "macos")]
fn write_macos(
    pasteboard: &objc2_app_kit::NSPasteboard,
    paths: &[String],
    mode: &str,
) -> Result<(), String> {
    use objc2::{rc::autoreleasepool, runtime::ProtocolObject};
    use objc2_app_kit::{NSPasteboardItem, NSPasteboardTypeFileURL};
    use objc2_foundation::{NSArray, NSData, NSString, NSURL};
    autoreleasepool(|_| {
        let mut items = Vec::new();
        for path in paths {
            let item = NSPasteboardItem::new();
            let url = NSURL::fileURLWithPath(&NSString::from_str(path));
            let value = url.absoluteString().ok_or("无法创建文件 URL")?;
            // NSPasteboard's type/data accessors read the first item. Keep the
            // action on each file item rather than adding a separate marker item.
            if !item.setString_forType(&value, unsafe { NSPasteboardTypeFileURL })
                || !item.setData_forType(
                    &NSData::with_bytes(mode.as_bytes()),
                    &NSString::from_str(MODE),
                )
            {
                return Err("无法创建文件剪贴板项目".into());
            }
            items.push(ProtocolObject::from_retained(item));
        }
        let items = NSArray::from_retained_slice(&items);
        pasteboard.clearContents();
        if !pasteboard.writeObjects(&items) {
            return Err("写入系统剪贴板失败".into());
        }
        Ok(())
    })
}

pub fn drag_paths(paths: Vec<String>) -> Result<Vec<std::path::PathBuf>, String> {
    let paths = local_paths(paths)?;
    if paths.is_empty() {
        return Err("请选择要拖动的项目".into());
    }
    paths
        .into_iter()
        .map(|p| {
            let path = Path::new(&p);
            std::fs::symlink_metadata(path).map_err(|e| format!("无法拖动「{p}」：{e}"))?;
            Ok(path.to_path_buf())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_os = "macos")]
    #[test]
    #[ignore = "requires macOS pasteboard server access; uses a private named pasteboard"]
    fn macos_file_items_keep_cut_mode_and_unicode_urls_together() {
        use objc2_app_kit::{NSPasteboard, NSPasteboardTypeFileURL};
        use objc2_foundation::NSString;
        let board = NSPasteboard::pasteboardWithUniqueName();
        let temp = tempfile::tempdir().unwrap();
        let paths: Vec<String> = ["中文 #%.txt", "second folder"]
            .iter()
            .map(|name| temp.path().join(name).to_string_lossy().into_owned())
            .collect();
        std::fs::write(&paths[0], "test").unwrap();
        std::fs::create_dir(&paths[1]).unwrap();
        write_macos(&board, &paths, "cut").unwrap();
        let items = board.pasteboardItems().unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(
            board
                .dataForType(&NSString::from_str(MODE))
                .unwrap()
                .to_vec(),
            b"cut"
        );
        let urls = items
            .iter()
            .map(|item| {
                assert_eq!(
                    item.dataForType(&NSString::from_str(MODE))
                        .unwrap()
                        .to_vec(),
                    b"cut"
                );
                item.stringForType(unsafe { NSPasteboardTypeFileURL })
                    .unwrap()
                    .to_string()
            })
            .collect();
        assert_eq!(local_paths(urls).unwrap(), paths);
        write_macos(&board, &paths[..1], "copy").unwrap();
        assert_eq!(board.pasteboardItems().unwrap().len(), 1);
        assert_eq!(
            board
                .dataForType(&NSString::from_str(MODE))
                .unwrap()
                .to_vec(),
            b"copy"
        );
        board.clearContents();
    }

    #[test]
    fn decodes_local_file_urls_and_deduplicates() {
        let path = std::env::temp_dir().join("剪贴板 #%.txt");
        let url = url::Url::from_file_path(&path).unwrap().to_string();
        assert_eq!(
            local_paths(vec![url, path.to_string_lossy().into()]).unwrap(),
            vec![path.to_string_lossy().into_owned()]
        );
        let directory = std::env::temp_dir().join("folder");
        let url = url::Url::from_directory_path(&directory)
            .unwrap()
            .to_string();
        assert_eq!(
            local_paths(vec![url, directory.to_string_lossy().into()]).unwrap(),
            vec![directory.to_string_lossy().into_owned()]
        );
    }

    #[test]
    fn rejects_relative_remote_and_missing_drag_sources() {
        assert!(local_paths(vec!["relative.txt".into()]).is_err());
        #[cfg(not(windows))]
        assert!(local_paths(vec!["file://remote/tmp/file".into()]).is_err());
        assert!(drag_paths(vec![]).is_err());
        let temp = tempfile::tempdir().unwrap();
        assert!(drag_paths(vec![temp.path().join("missing").to_string_lossy().into()]).is_err());
    }
}
