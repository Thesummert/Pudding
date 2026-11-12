use std::process::Command;

use crate::filesystem;

pub fn open(path: &str, terminal: &str, custom: Option<&str>) -> Result<(), String> {
    let directory = filesystem::resolve_directory(path)?;
    let mut command = if terminal == "custom" {
        custom_command(custom)?
    } else {
        command(terminal)?
    };
    command.current_dir(&directory);
    #[cfg(target_os = "macos")]
    add_macos_directory(&mut command, &directory, terminal, custom);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x00000010); // CREATE_NEW_CONSOLE
        if terminal == "windows-terminal"
            || (terminal == "custom"
                && custom.is_some_and(|path| path.to_ascii_lowercase().ends_with("wt.exe")))
        {
            command.arg("-d").arg(&directory);
        }
    }
    #[cfg(target_os = "macos")]
    {
        let status = command
            .status()
            .map_err(|error| format!("无法打开终端：{error}"))?;
        if !status.success() {
            return Err("无法打开所选终端，请确认该应用已安装。".into());
        }
    }
    #[cfg(not(target_os = "macos"))]
    command
        .spawn()
        .map_err(|error| format!("无法打开所选终端，请确认该应用已安装：{error}"))?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn add_macos_directory(
    command: &mut Command,
    directory: &std::path::Path,
    terminal: &str,
    custom: Option<&str>,
) {
    let is_alacritty = terminal == "custom"
        && custom.is_some_and(|path| {
            plist::Value::from_file(std::path::Path::new(path).join("Contents/Info.plist"))
                .ok()
                .and_then(|value| {
                    value
                        .as_dictionary()?
                        .get("CFBundleIdentifier")?
                        .as_string()
                        .map(str::to_owned)
                })
                .is_some_and(|identifier| identifier.eq_ignore_ascii_case("org.alacritty"))
        });
    if is_alacritty {
        // A fresh app instance is required for Launch Services to pass --args.
        command
            .arg("-n")
            .arg("--args")
            .arg("--working-directory")
            .arg(directory);
    } else {
        command.arg(directory);
    }
}

pub fn choose() -> Option<String> {
    let dialog = rfd::FileDialog::new().set_title("选择终端应用");
    #[cfg(target_os = "macos")]
    let dialog = dialog
        .set_directory("/Applications")
        .add_filter("应用程序", &["app"]);
    #[cfg(target_os = "windows")]
    let dialog = dialog.add_filter("应用程序", &["exe"]);
    dialog
        .pick_file()
        .map(|path| path.to_string_lossy().into_owned())
}

fn custom_command(custom: Option<&str>) -> Result<Command, String> {
    let path = std::path::Path::new(
        custom
            .filter(|path| !path.is_empty())
            .ok_or("请先选择终端应用")?,
    );
    if !path.is_absolute() || !path.exists() {
        return Err("找不到所选终端应用，请重新选择。".into());
    }
    #[cfg(target_os = "macos")]
    {
        if path.extension().and_then(|value| value.to_str()) != Some("app") || !path.is_dir() {
            return Err("请选择应用程序（.app）。".into());
        }
        let mut command = Command::new("open");
        command.arg("-a").arg(path);
        Ok(command)
    }
    #[cfg(not(target_os = "macos"))]
    {
        if !path.is_file() {
            return Err("请选择终端可执行文件。".into());
        }
        Ok(Command::new(path))
    }
}

fn command(terminal: &str) -> Result<Command, String> {
    #[cfg(target_os = "macos")]
    let executable = match terminal {
        "system" | "terminal" => "Terminal",
        "iterm" => "iTerm",
        "warp" => "Warp",
        _ => return Err("不支持所选终端".into()),
    };
    #[cfg(target_os = "macos")]
    {
        let mut command = Command::new("open");
        command.arg("-a").arg(executable);
        Ok(command)
    }
    #[cfg(target_os = "linux")]
    {
        let executable = match terminal {
            "system" => "x-terminal-emulator",
            "gnome-terminal" => "gnome-terminal",
            "konsole" => "konsole",
            "xfce4-terminal" => "xfce4-terminal",
            "xterm" => "xterm",
            _ => return Err("不支持所选终端".into()),
        };
        Ok(Command::new(executable))
    }
    #[cfg(target_os = "windows")]
    {
        let executable = match terminal {
            "system" | "cmd" => "cmd.exe",
            "windows-terminal" => "wt.exe",
            "powershell" => "powershell.exe",
            _ => return Err("不支持所选终端".into()),
        };
        Ok(Command::new(executable))
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        let _ = terminal;
        Err("当前系统不支持打开终端".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_os = "macos")]
    #[test]
    fn alacritty_receives_working_directory_as_argument() {
        let bundle = tempfile::tempdir().unwrap();
        std::fs::create_dir(bundle.path().join("Contents")).unwrap();
        let mut info = plist::Dictionary::new();
        info.insert(
            "CFBundleIdentifier".into(),
            plist::Value::String("org.alacritty".into()),
        );
        plist::Value::Dictionary(info)
            .to_file_xml(bundle.path().join("Contents/Info.plist"))
            .unwrap();
        let mut command = Command::new("open");
        add_macos_directory(
            &mut command,
            std::path::Path::new("/tmp/a folder"),
            "custom",
            bundle.path().to_str(),
        );
        assert_eq!(
            command.get_args().collect::<Vec<_>>(),
            ["-n", "--args", "--working-directory", "/tmp/a folder"]
        );
    }

    #[test]
    fn rejects_invalid_terminal_and_directory() {
        assert!(command("unknown").is_err());
        assert!(custom_command(None).is_err());
        assert!(custom_command(Some("relative/path")).is_err());
        assert!(open("/this-directory-does-not-exist-pudding", "system", None).is_err());
    }
}
