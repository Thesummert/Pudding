use crate::filesystem::{self, FileEntry};
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use std::{
    collections::HashMap,
    fs,
    io::Read,
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tauri::{AppHandle, Emitter};

#[derive(Default)]
pub struct SearchState {
    watcher: Mutex<Option<RecommendedWatcher>>,
    cache: Arc<Mutex<HashMap<(String, bool), Vec<Indexed>>>>,
}
#[derive(Clone)]
struct Indexed {
    entry: FileEntry,
    text: String,
}

pub fn watch(state: &SearchState, app: AppHandle, path: String) -> Result<(), String> {
    let root = filesystem::resolve_directory(&path)?;
    let cache = state.cache.clone();
    let mut watcher = notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
        if event.is_ok() {
            if let Ok(mut cache) = cache.lock() {
                cache.clear();
            }
            let _ = app.emit("filesystem-changed", ());
        }
    })
    .map_err(|e| e.to_string())?;
    watcher
        .watch(&root, RecursiveMode::Recursive)
        .map_err(|e| format!("无法监听文件夹：{e}"))?;
    *state.watcher.lock().map_err(|e| e.to_string())? = Some(watcher);
    state.cache.lock().map_err(|e| e.to_string())?.clear();
    Ok(())
}

pub fn search(
    state: &SearchState,
    path: String,
    query: String,
    hidden: bool,
    search_content: bool,
    mut progress: impl FnMut(usize),
) -> Result<Vec<FileEntry>, String> {
    let root = filesystem::resolve_directory(&path)?;
    let key = (root.to_string_lossy().to_string(), search_content);
    // Hold the lock while building so a filesystem event cannot leave a stale index cached.
    let mut cache = state.cache.lock().map_err(|e| e.to_string())?;
    if !cache.contains_key(&key) {
        let mut index = Vec::new();
        let mut pending = vec![(root, false)];
        let mut last_report = std::time::Instant::now();
        progress(0);
        while let Some((dir, parent_hidden)) = pending.pop() {
            let Ok(listing) = filesystem::read_directory(&dir.to_string_lossy()) else {
                continue;
            };
            for mut entry in listing.entries {
                entry.hidden |= parent_hidden;
                if entry.is_dir && !entry.is_symlink {
                    pending.push((PathBuf::from(&entry.path), entry.hidden));
                }
                let mut text = String::new();
                if search_content
                    && !entry.is_dir
                    && !entry.is_symlink
                    && entry.size <= 2 * 1024 * 1024
                {
                    if let Ok(file) = fs::File::open(&entry.path) {
                        let mut bytes = Vec::new();
                        if file
                            .take(2 * 1024 * 1024 + 1)
                            .read_to_end(&mut bytes)
                            .is_ok()
                            && bytes.len() <= 2 * 1024 * 1024
                            && !bytes.contains(&0)
                        {
                            if let Ok(value) = String::from_utf8(bytes) {
                                text = value.to_lowercase();
                            }
                        }
                    }
                }
                index.push(Indexed { entry, text });
                if last_report.elapsed() >= std::time::Duration::from_millis(100) {
                    progress(index.len());
                    last_report = std::time::Instant::now();
                }
            }
        }
        progress(index.len());
        cache.insert(key.clone(), index);
    }
    let term = query.trim().to_lowercase();
    Ok(cache[&key]
        .iter()
        .filter(|item| {
            (hidden || !item.entry.hidden)
                && (item.entry.name.to_lowercase().contains(&term)
                    || (search_content && item.text.contains(&term)))
        })
        .map(|item| item.entry.clone())
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn finds_nested_names_and_text_and_respects_hidden_directories() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("nested")).unwrap();
        fs::create_dir_all(dir.path().join(".private")).unwrap();
        fs::write(dir.path().join("nested/note.txt"), "Hello searchable world").unwrap();
        fs::write(dir.path().join(".private/secret.txt"), "searchable").unwrap();
        let state = SearchState::default();
        let path = dir.path().to_string_lossy().to_string();
        assert!(search(
            &state,
            path.clone(),
            "searchable".into(),
            false,
            false,
            |_| {}
        )
        .unwrap()
        .is_empty());
        assert_eq!(
            search(
                &state,
                path.clone(),
                "SEARCHABLE".into(),
                false,
                true,
                |_| {}
            )
            .unwrap()
            .len(),
            1
        );
        assert_eq!(
            search(
                &state,
                path.clone(),
                "searchable".into(),
                true,
                true,
                |_| {}
            )
            .unwrap()
            .len(),
            2
        );
        assert_eq!(
            search(&state, path, "note.txt".into(), false, false, |_| {})
                .unwrap()
                .len(),
            1
        );
    }
}
