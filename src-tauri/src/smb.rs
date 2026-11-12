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
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (url, username, password);
        Err("当前仅支持在 macOS 上连接 SMB 共享".into())
    }
}

pub async fn list_shares(
    address: &str,
    username: &str,
    password: &str,
) -> Result<Vec<String>, String> {
    let url = validate_address(address)?;
    #[cfg(target_os = "macos")]
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
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (url, username, password);
        Err("当前仅支持在 macOS 上连接 SMB 共享".into())
    }
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
}
