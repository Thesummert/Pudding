use std::path::Path;

pub fn checked_files(paths: Vec<String>) -> Result<Vec<String>, String> {
    if paths.is_empty() {
        return Err("请选择要分享的文件".into());
    }
    paths
        .into_iter()
        .map(|path| {
            let path = crate::filesystem::openable_path(&path)?;
            if !Path::new(&path).is_file() {
                return Err("只能分享文件，请取消选择文件夹".into());
            }
            Ok(path)
        })
        .collect()
}

#[cfg(target_os = "macos")]
thread_local! {
    // AppKit's picker must remain alive while the popover is open.
    static PICKER: std::cell::RefCell<Option<objc2::rc::Retained<objc2_app_kit::NSSharingServicePicker>>> = const { std::cell::RefCell::new(None) };
}

#[cfg(target_os = "macos")]
pub fn show(window: &tauri::WebviewWindow, paths: Vec<String>) -> Result<(), String> {
    use objc2::{rc::Retained, AnyThread};
    use objc2_app_kit::{NSSharingServicePicker, NSWindow};
    use objc2_foundation::{NSArray, NSRectEdge, NSString, NSURL};
    let raw = window.ns_window().map_err(|e| e.to_string())?;
    let native = unsafe { &*(raw as *const NSWindow) };
    let view = native.contentView().ok_or("无法获取窗口视图")?;
    let urls: Vec<Retained<NSURL>> = paths
        .iter()
        .map(|path| NSURL::fileURLWithPath(&NSString::from_str(path)))
        .collect();
    let items = NSArray::from_retained_slice(&urls);
    // NSArray is covariant here: each NSURL is an Objective-C object.
    let untyped_items = unsafe { &*(&*items as *const NSArray<NSURL> as *const NSArray) };
    let picker = unsafe {
        NSSharingServicePicker::initWithItems(NSSharingServicePicker::alloc(), untyped_items)
    };
    picker.showRelativeToRect_ofView_preferredEdge(view.bounds(), &view, NSRectEdge::MinY);
    PICKER.with(|stored| *stored.borrow_mut() = Some(picker));
    Ok(())
}

#[cfg(target_os = "linux")]
pub fn show(_window: &tauri::WebviewWindow, paths: Vec<String>) -> Result<(), String> {
    let mut command = std::process::Command::new("xdg-email");
    for path in paths {
        command.arg("--attach").arg(path);
    }
    command
        .spawn()
        .map_err(|e| format!("无法打开系统邮件分享（需要 xdg-email）：{e}"))?;
    Ok(())
}

#[cfg(target_os = "windows")]
pub fn show(window: &tauri::WebviewWindow, paths: Vec<String>) -> Result<(), String> {
    use windows::{
        core::{factory, Interface, HSTRING},
        ApplicationModel::DataTransfer::{DataRequestedEventArgs, DataTransferManager},
        Foundation::TypedEventHandler,
        Storage::{IStorageItem, StorageFile},
        Win32::{Foundation::HWND, UI::Shell::IDataTransferManagerInterop},
    };
    use windows_collections::IIterable;
    let result = || -> windows::core::Result<()> {
        let hwnd = HWND(
            window
                .hwnd()
                .map_err(|e| {
                    windows::core::Error::new(
                        windows::core::HRESULT(0x80004005u32 as i32),
                        e.to_string(),
                    )
                })?
                .0 as _,
        );
        let interop: IDataTransferManagerInterop =
            factory::<DataTransferManager, IDataTransferManagerInterop>()?;
        let manager: DataTransferManager = unsafe { interop.GetForWindow(hwnd)? };
        SHARE_HANDLER.with(|slot| {
            if let Some((old, token)) = slot.borrow_mut().take() {
                let _ = old.RemoveDataRequested(token);
            }
            let handler = TypedEventHandler::<DataTransferManager, DataRequestedEventArgs>::new(
                move |_, args| {
                    let files: Vec<Option<IStorageItem>> = paths
                        .iter()
                        .map(|path| {
                            Ok(Some(
                                StorageFile::GetFileFromPathAsync(&HSTRING::from(path))?
                                    .get()?
                                    .cast()?,
                            ))
                        })
                        .collect::<windows::core::Result<_>>()?;
                    let items: IIterable<IStorageItem> = files.into();
                    let data = args
                        .as_ref()
                        .ok_or(windows::core::Error::from(windows::core::HRESULT(
                            0x80004003u32 as i32,
                        )))?
                        .Request()?
                        .Data()?;
                    data.Properties()?.SetTitle(&HSTRING::from("分享文件"))?;
                    data.SetStorageItems(&items, true)
                },
            );
            let token = manager.DataRequested(&handler)?;
            *slot.borrow_mut() = Some((manager, token));
            Ok::<(), windows::core::Error>(())
        })?;
        unsafe { interop.ShowShareUIForWindow(hwnd) }
    };
    result().map_err(|e| format!("无法打开系统分享：{e}"))
}

#[cfg(target_os = "windows")]
thread_local! {
    static SHARE_HANDLER: std::cell::RefCell<Option<(windows::ApplicationModel::DataTransfer::DataTransferManager, i64)>> = const { std::cell::RefCell::new(None) };
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn validates_share_selection() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("文 件.txt");
        std::fs::write(&file, "test").unwrap();
        assert_eq!(
            checked_files(vec![file.to_string_lossy().into_owned()])
                .unwrap()
                .len(),
            1
        );
        assert!(checked_files(vec![]).is_err());
        assert!(checked_files(vec![dir.path().to_string_lossy().into_owned()]).is_err());
        assert!(checked_files(vec!["relative.txt".into()]).is_err());
    }
}
