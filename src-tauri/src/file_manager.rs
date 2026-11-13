use std::{path::Path, process::Command};

use crate::filesystem;

fn command(directory: &Path) -> Result<Command, String> {
    #[cfg(target_os = "macos")]
    let mut command = {
        let mut command = Command::new("/usr/bin/open");
        command.args(["-a", "Finder", "--"]);
        command
    };
    #[cfg(target_os = "windows")]
    let mut command = Command::new("explorer.exe");
    #[cfg(target_os = "linux")]
    let mut command = Command::new("xdg-open");
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        let _ = directory;
        return Err("当前系统不支持打开系统文件管理器".into());
    }
    #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
    {
        command.arg(directory);
        Ok(command)
    }
}

pub fn open(path: &str) -> Result<(), String> {
    let directory = filesystem::resolve_directory(path)?;
    #[cfg(target_os = "windows")]
    let directory = std::path::PathBuf::from(filesystem::display(&directory));
    let mut command = command(&directory)?;
    #[cfg(target_os = "macos")]
    {
        let status = command
            .status()
            .map_err(|error| format!("无法打开系统文件管理器：{error}"))?;
        if !status.success() {
            return Err("无法在 Finder 中打开文件夹".into());
        }
    }
    #[cfg(not(target_os = "macos"))]
    command
        .spawn()
        .map_err(|error| format!("无法打开系统文件管理器：{error}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passes_the_directory_as_one_argument() {
        let directory = Path::new("/tmp/共享 文件夹");
        let command = command(directory).unwrap();
        let args: Vec<_> = command.get_args().collect();
        assert_eq!(args.last().copied(), Some(directory.as_os_str()));
        #[cfg(target_os = "macos")]
        {
            assert_eq!(command.get_program(), "/usr/bin/open");
            assert_eq!(&args[..3], ["-a", "Finder", "--"]);
        }
    }

    #[test]
    fn rejects_nonexistent_and_relative_directories() {
        assert!(open("relative-directory").is_err());
        let temp = tempfile::tempdir().unwrap();
        assert!(open(&temp.path().join("missing").to_string_lossy()).is_err());
    }
}
