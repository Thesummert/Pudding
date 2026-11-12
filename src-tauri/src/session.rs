use std::{
    collections::HashMap,
    fs::{self, File, OpenOptions},
    io,
    path::{Path, PathBuf},
    sync::Mutex,
};

use crate::filesystem;

pub struct Session {
    _lock: Option<File>,
    last_path: Option<PathBuf>,
    restore: bool,
    windows: Mutex<HashMap<String, String>>,
}

impl Session {
    pub fn new(data_dir: &Path) -> io::Result<Self> {
        fs::create_dir_all(data_dir)?;
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(data_dir.join("instances.lock"))?;
        let restore = acquire_instance_lock(&lock)?;
        Ok(Self {
            _lock: Some(lock),
            last_path: Some(data_dir.join("last-directory")),
            restore,
            windows: Mutex::new(HashMap::new()),
        })
    }

    pub fn disabled() -> Self {
        Self {
            _lock: None,
            last_path: None,
            restore: false,
            windows: Mutex::new(HashMap::new()),
        }
    }

    pub fn startup_path(&self) -> Option<String> {
        if !self.restore {
            return None;
        }
        let path = fs::read_to_string(self.last_path.as_ref()?).ok()?;
        filesystem::read_directory(&path).ok()?;
        Some(path)
    }

    pub fn set_directory(&self, label: &str, path: String) {
        if let Ok(mut windows) = self.windows.lock() {
            windows.insert(label.to_owned(), path);
        }
    }

    pub fn close_window(&self, label: &str) {
        if let Ok(mut windows) = self.windows.lock() {
            if let (Some(path), Some(file)) = (windows.remove(label), &self.last_path) {
                if let Err(error) = fs::write(file, path) {
                    eprintln!("无法保存最后关闭的目录：{error}");
                }
            }
        }
    }
}

// Every process holds a shared lock for its lifetime. An exclusive lock can
// only be acquired when no other Pudding process is running.
#[cfg(unix)]
fn acquire_instance_lock(file: &File) -> io::Result<bool> {
    use std::os::fd::AsRawFd;
    let fd = file.as_raw_fd();
    let exclusive = unsafe { libc::flock(fd, libc::LOCK_EX | libc::LOCK_NB) } == 0;
    if !exclusive {
        let error = io::Error::last_os_error();
        if error.kind() != io::ErrorKind::WouldBlock {
            return Err(error);
        }
    }
    if unsafe { libc::flock(fd, libc::LOCK_SH) } != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(exclusive)
}

#[cfg(windows)]
fn acquire_instance_lock(file: &File) -> io::Result<bool> {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::{
        Foundation::ERROR_LOCK_VIOLATION,
        Storage::FileSystem::{
            LockFileEx, UnlockFileEx, LOCKFILE_EXCLUSIVE_LOCK, LOCKFILE_FAIL_IMMEDIATELY,
        },
        System::IO::OVERLAPPED,
    };
    let handle = file.as_raw_handle() as _;
    let mut overlapped = OVERLAPPED::default();
    let exclusive = unsafe {
        LockFileEx(
            handle,
            LOCKFILE_EXCLUSIVE_LOCK | LOCKFILE_FAIL_IMMEDIATELY,
            0,
            1,
            0,
            &mut overlapped,
        )
    } != 0;
    if exclusive {
        if unsafe { UnlockFileEx(handle, 0, 1, 0, &mut overlapped) } == 0 {
            return Err(io::Error::last_os_error());
        }
    } else {
        let error = io::Error::last_os_error();
        if error.raw_os_error() != Some(ERROR_LOCK_VIOLATION as i32) {
            return Err(error);
        }
    }
    if unsafe { LockFileEx(handle, 0, 0, 1, 0, &mut overlapped) } == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(exclusive)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restores_last_closed_directory_only_without_another_instance() {
        let dir = tempfile::tempdir().unwrap();
        let first = Session::new(dir.path()).unwrap();
        assert_eq!(first.startup_path(), None);
        first.set_directory("main", dir.path().to_string_lossy().into_owned());
        first.close_window("main");
        let second = Session::new(dir.path()).unwrap();
        assert_eq!(second.startup_path(), None);
        drop(first);
        let third = Session::new(dir.path()).unwrap();
        assert_eq!(third.startup_path(), None);
        drop(second);
        drop(third);
        let next = Session::new(dir.path()).unwrap();
        assert_eq!(
            next.startup_path(),
            Some(dir.path().to_string_lossy().into_owned())
        );
        fs::write(
            dir.path().join("last-directory"),
            dir.path().join("missing").to_string_lossy().as_bytes(),
        )
        .unwrap();
        assert_eq!(next.startup_path(), None);
    }
}
