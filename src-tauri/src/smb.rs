use url::Url;

fn is_guest(username: &str, password: &str) -> bool {
    password.is_empty() && (username.trim().is_empty() || username.eq_ignore_ascii_case("guest"))
}

fn validate_address(address: &str) -> Result<Url, String> {
    let address = address.trim();
    let url =
        Url::parse(address).map_err(|_| "请输入有效的 SMB 地址，例如 smb://server".to_string())?;
    if url.scheme() != "smb"
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(
            "请输入 smb://服务器或 smb://服务器/共享文件夹，不要在地址中包含用户名或密码".into(),
        );
    }
    Ok(url)
}

pub fn connect(address: &str, username: &str, password: &str) -> Result<Vec<String>, String> {
    let url = validate_address(address)?;
    if url.path().trim_matches('/').is_empty() {
        return Err("请先选择共享文件夹".into());
    }
    #[cfg(target_os = "macos")]
    {
        use core_foundation::{
            array::{CFArray, CFArrayRef},
            base::TCFType,
            boolean::CFBoolean,
            dictionary::CFMutableDictionary,
            string::{CFString, CFStringRef},
            url::{CFURLCreateWithString, CFURLRef, CFURL},
        };
        use std::ptr;
        #[link(name = "NetFS", kind = "framework")]
        unsafe extern "C" {
            fn NetFSMountURLSync(
                url: CFURLRef,
                mountpath: CFURLRef,
                user: CFStringRef,
                password: CFStringRef,
                open_options: *mut std::ffi::c_void,
                mount_options: *mut std::ffi::c_void,
                mountpoints: *mut CFArrayRef,
            ) -> i32;
        }
        let url_string = CFString::new(url.as_str());
        let url_ref = unsafe {
            CFURLCreateWithString(ptr::null(), url_string.as_concrete_TypeRef(), ptr::null())
        };
        if url_ref.is_null() {
            return Err("无法解析 SMB 地址".into());
        }
        let url = unsafe { CFURL::wrap_under_create_rule(url_ref) };
        let guest = is_guest(username, password);
        let user = CFString::new(username);
        let has_password = !password.is_empty();
        let password = CFString::new(password);
        let mut mountpoints: CFArrayRef = ptr::null();
        let mut options = CFMutableDictionary::new();
        options.add(
            &CFString::new("UIOption"),
            &CFString::new("NoUI").as_CFType(),
        );
        if guest {
            options.add(
                &CFString::new("Guest"),
                &CFBoolean::true_value().as_CFType(),
            );
        }
        let status = unsafe {
            NetFSMountURLSync(
                url.as_concrete_TypeRef(),
                ptr::null(),
                if guest || username.is_empty() {
                    ptr::null()
                } else {
                    user.as_concrete_TypeRef()
                },
                if guest || (username.is_empty() && !has_password) {
                    ptr::null()
                } else {
                    password.as_concrete_TypeRef()
                },
                options.as_concrete_TypeRef() as *mut std::ffi::c_void,
                ptr::null_mut(),
                &mut mountpoints,
            )
        };
        let mounts = if mountpoints.is_null() {
            Vec::new()
        } else {
            let array: CFArray<CFString> = unsafe { CFArray::wrap_under_create_rule(mountpoints) };
            array.iter().map(|path| path.to_string()).collect()
        };
        if status != 0 {
            return Err(if status == -128 {
                "连接已取消".into()
            } else if status == 80 {
                "认证失败：请检查账户、密码及共享的访客访问权限。".into()
            } else if status > 0 {
                format!("连接失败：{}", std::io::Error::from_raw_os_error(status))
            } else {
                format!("连接失败（系统错误 {status}）")
            });
        }
        if mounts.is_empty() {
            return Err("系统未返回共享的挂载位置".into());
        }
        Ok(mounts)
    }
    #[cfg(target_os = "linux")]
    {
        linux_connect(&url, username, password)
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        let _ = (url, username, password);
        Err("当前系统不支持连接 SMB 共享".into())
    }
}

pub async fn list_shares(
    address: &str,
    username: &str,
    password: &str,
) -> Result<Vec<String>, String> {
    let url = validate_address(address)?;
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        let host = url.host().ok_or("请输入服务器地址")?;
        let server = format!("{}:{}", host, url.port().unwrap_or(445));
        let mut client = smb2::connect(&server, username, password)
            .await
            .map_err(|e| format!("无法连接 SMB 服务器：{e}"))?;
        let shares = client
            .list_shares()
            .await
            .map_err(|e| format!("无法读取共享列表：{e}"))?;
        Ok(shares.into_iter().map(|share| share.name).collect())
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        let _ = (url, username, password);
        Err("当前系统不支持连接 SMB 共享".into())
    }
}

#[cfg(target_os = "linux")]
fn linux_connect(url: &Url, username: &str, password: &str) -> Result<Vec<String>, String> {
    use gio::prelude::*;
    use gtk::{gio, glib};

    let context = glib::MainContext::new();
    context
        .with_thread_default(|| {
            let file = gio::File::for_uri(url.as_str());
            let operation = gio::MountOperation::new();
            let username = username.to_owned();
            let password = password.to_owned();
            let requested = std::cell::Cell::new(false);
            operation.connect_ask_password(move |op, _, _, default_domain, _| {
                if requested.replace(true) {
                    op.reply(gio::MountOperationResult::Aborted);
                    return;
                }
                op.set_anonymous(is_guest(&username, &password));
                if !is_guest(&username, &password) {
                    op.set_username(Some(&username));
                    op.set_password(Some(&password));
                    op.set_domain(Some(default_domain));
                }
                op.set_password_save(gio::PasswordSave::Never);
                op.reply(gio::MountOperationResult::Handled);
            });
            let result = context.block_on(
                file.mount_enclosing_volume_future(gio::MountMountFlags::NONE, Some(&operation)),
            );
            if let Err(error) = result {
                if !error.matches(gio::IOErrorEnum::AlreadyMounted) {
                    return Err(format!(
                        "无法挂载 SMB 共享：{error}（请确认已安装并运行 GVfs SMB 服务）"
                    ));
                }
            }
            gvfs_path(url).map(|path| vec![path])
        })
        .map_err(|e| format!("无法初始化挂载服务：{e}"))?
}

#[cfg(target_os = "linux")]
fn gvfs_path(url: &Url) -> Result<String, String> {
    let runtime = std::env::var_os("XDG_RUNTIME_DIR")
        .ok_or("未设置 XDG_RUNTIME_DIR，无法访问 GVfs 挂载目录")?;
    gvfs_path_in(url, &std::path::Path::new(&runtime).join("gvfs"))
}

#[cfg(target_os = "linux")]
fn gvfs_path_in(url: &Url, mount_dir: &std::path::Path) -> Result<String, String> {
    use gtk::glib;
    let decode = |s: &str| glib::uri_unescape_string(s, None::<&str>).map(|v| v.to_string());
    let host = url.host_str().ok_or("缺少服务器地址")?;
    let mut segments = url.path_segments().ok_or("缺少共享名称")?;
    let share = decode(segments.next().unwrap_or_default()).ok_or("共享名称无效")?;
    let entries = std::fs::read_dir(mount_dir)
        .map_err(|e| format!("无法读取 GVfs 挂载目录：{e}；请确认 gvfs-fuse 已运行"))?;
    let mut matches = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        let Some(fields) = name.strip_prefix("smb-share:") else {
            continue;
        };
        let mut server = None;
        let mut mounted_share = None;
        let mut port = None;
        for field in fields.split(',') {
            if let Some((key, value)) = field.split_once('=') {
                match key {
                    "server" => server = decode(value),
                    "share" => mounted_share = decode(value),
                    "port" => port = value.parse::<u16>().ok(),
                    _ => {}
                }
            }
        }
        if server
            .as_deref()
            .is_some_and(|s| s.eq_ignore_ascii_case(host))
            && mounted_share
                .as_deref()
                .is_some_and(|s| s.to_lowercase() == share.to_lowercase())
            && port.unwrap_or(445) == url.port().unwrap_or(445)
        {
            matches.push(entry.path());
        }
    }
    if matches.len() > 1 {
        return Err("存在多个相同服务器和共享的挂载，请先卸载多余的连接".into());
    }
    if let Some(mut path) = matches.pop() {
        for segment in segments {
            let segment = decode(segment).ok_or("路径无效")?;
            if segment == "." || segment == ".." || segment.contains('/') || segment.contains('\0')
            {
                return Err("路径无效".into());
            }
            path.push(segment);
        }
        if path.exists() {
            return Ok(path.to_string_lossy().into_owned());
        }
        return Err("共享已挂载，但指定的目录不存在".into());
    }

    Err("共享已挂载，但未找到本地 GVfs 路径；请确认 gvfs-fuse 已安装并运行".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_smb_share_addresses_without_credentials() {
        assert!(validate_address("smb://server/share").is_ok());
        assert!(validate_address("smb://server").is_ok());
        assert!(validate_address("smb://server/").is_ok());
        assert!(validate_address("smb://192.168.1.2/共享").is_ok());
        for address in [
            "https://server/share",
            "smb://user:password@server/share",
            "smb://server/share?x=1",
            "smb://server/share#x",
            "server/share",
        ] {
            assert!(validate_address(address).is_err(), "{address}");
        }
    }

    #[test]
    fn detects_explicit_or_empty_guest_credentials() {
        assert!(is_guest("", ""));
        assert!(is_guest("Guest", ""));
        assert!(!is_guest("alice", ""));
        assert!(!is_guest("", "password"));
        assert!(!is_guest("guest", "password"));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn finds_gvfs_share_with_different_case_and_encoded_space() {
        let dir = tempfile::tempdir().unwrap();
        let mount = dir
            .path()
            .join("smb-share:server=192.168.0.1,share=u30%20air");
        std::fs::create_dir(&mount).unwrap();
        std::fs::create_dir(mount.join("Photos")).unwrap();
        let url = Url::parse("smb://192.168.0.1/U30%20Air/Photos").unwrap();
        assert_eq!(
            gvfs_path_in(&url, dir.path()).unwrap(),
            mount.join("Photos").to_str().unwrap()
        );
        let other = Url::parse("smb://192.168.0.1/another").unwrap();
        assert!(gvfs_path_in(&other, dir.path()).is_err());
    }
}
