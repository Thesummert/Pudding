use std::{io, path::Path};

/// Rename atomically without replacing a destination created after preflight.
/// Unsupported filesystems fail safely instead of falling back to an overwriting rename.
pub fn exclusive(source: &Path, destination: &Path) -> io::Result<()> {
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        use std::{ffi::CString, os::unix::ffi::OsStrExt};
        let source = CString::new(source.as_os_str().as_bytes())?;
        let destination = CString::new(destination.as_os_str().as_bytes())?;
        // SAFETY: Both pointers reference valid, NUL-terminated paths for the call.
        // The exclusive flag makes an existing destination an error.
        #[cfg(target_os = "macos")]
        let result =
            unsafe { libc::renamex_np(source.as_ptr(), destination.as_ptr(), libc::RENAME_EXCL) };
        #[cfg(target_os = "linux")]
        let result = unsafe {
            libc::renameat2(
                libc::AT_FDCWD,
                source.as_ptr(),
                libc::AT_FDCWD,
                destination.as_ptr(),
                libc::RENAME_NOREPLACE,
            )
        };
        if result != 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        let source: Vec<_> = source.as_os_str().encode_wide().chain(Some(0)).collect();
        let destination: Vec<_> = destination
            .as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect();
        // SAFETY: Both UTF-16 paths are NUL-terminated and valid for the call.
        // Zero flags excludes MOVEFILE_REPLACE_EXISTING and cross-volume copying.
        let result = unsafe {
            windows_sys::Win32::Storage::FileSystem::MoveFileExW(
                source.as_ptr(),
                destination.as_ptr(),
                0,
            )
        };
        if result == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", windows)))]
    {
        let _ = (source, destination);
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "此系统暂不支持安全重命名",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn rejects_a_destination_even_without_a_preflight_check() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source");
        let destination = dir.path().join("destination");
        fs::write(&source, "source content").unwrap();
        fs::write(&destination, "existing content").unwrap();
        assert!(exclusive(&source, &destination).is_err());
        assert_eq!(fs::read_to_string(&source).unwrap(), "source content");
        assert_eq!(
            fs::read_to_string(&destination).unwrap(),
            "existing content"
        );
    }
}
