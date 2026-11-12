use std::{ffi::OsString, path::PathBuf, sync::Mutex};

#[derive(Default)]
pub struct OpenRequests(Mutex<Vec<String>>);

impl OpenRequests {
    pub fn push(&self, paths: impl IntoIterator<Item = PathBuf>) -> bool {
        let mut pending = self.0.lock().unwrap_or_else(|error| error.into_inner());
        let before = pending.len();
        pending.extend(
            paths
                .into_iter()
                .filter(|path| path.is_absolute())
                .map(|path| path.to_string_lossy().into_owned()),
        );
        pending.len() != before
    }

    pub fn take(&self) -> Vec<String> {
        std::mem::take(&mut *self.0.lock().unwrap_or_else(|error| error.into_inner()))
    }
}

pub fn local_url(url: &url::Url) -> Option<PathBuf> {
    if url.scheme() != "file" || url.host_str().is_some_and(|host| host != "localhost") {
        return None;
    }
    url.to_file_path().ok().filter(|path| path.is_absolute())
}

pub fn arguments(args: impl IntoIterator<Item = OsString>) -> Vec<PathBuf> {
    args.into_iter()
        .filter_map(|arg| {
            let path = PathBuf::from(&arg);
            if path.is_absolute() {
                return Some(path);
            }
            let text = arg.to_str()?;
            local_url(&url::Url::parse(text).ok()?)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_local_file_urls_and_absolute_arguments_only() {
        let dir = tempfile::tempdir().unwrap();
        let uri = url::Url::from_directory_path(dir.path()).unwrap();
        assert_eq!(local_url(&uri), Some(dir.path().to_path_buf()));
        assert!(local_url(&url::Url::parse("https://example.org/folder").unwrap()).is_none());
        assert!(local_url(&url::Url::parse("file://remote-host/path").unwrap()).is_none());
        let paths = arguments([
            dir.path().as_os_str().to_owned(),
            "--flag".into(),
            "relative".into(),
        ]);
        assert_eq!(paths, vec![dir.path().to_path_buf()]);
    }

    #[test]
    fn queues_requests_until_consumed() {
        let queue = OpenRequests::default();
        let dir = tempfile::tempdir().unwrap();
        assert!(queue.push([dir.path().to_path_buf()]));
        assert!(!queue.push([PathBuf::from("relative")]));
        assert_eq!(queue.take(), vec![dir.path().to_string_lossy().to_string()]);
        assert!(queue.take().is_empty());
    }
}
