use std::{
    env, fs,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::Mutex,
    time::{Duration, Instant},
};

use serde::Deserialize;
use tauri::{Manager, Resource, ResourceId, Webview};
use tempfile::TempDir;

use crate::{blocking, filesystem};

const MAX_BYTES: u64 = 512 * 1024 * 1024;
const TIMEOUT: Duration = Duration::from_secs(180);

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum PreviewFormat {
    Mp4,
    Webm,
}

impl PreviewFormat {
    fn output(self) -> (&'static str, &'static [&'static str]) {
        match self {
            Self::Mp4 => (
                "preview.mp4",
                &[
                    "-c:v",
                    "libx264",
                    "-preset",
                    "ultrafast",
                    "-profile:v",
                    "baseline",
                    "-crf",
                    "24",
                    "-c:a",
                    "aac",
                    "-movflags",
                    "+faststart",
                ],
            ),
            Self::Webm => (
                "preview.webm",
                &[
                    "-c:v",
                    "libvpx",
                    "-deadline",
                    "realtime",
                    "-cpu-used",
                    "5",
                    "-b:v",
                    "2M",
                    "-c:a",
                    "libopus",
                ],
            ),
        }
    }
}

fn ffmpeg() -> Option<PathBuf> {
    let name = if cfg!(windows) {
        "ffmpeg.exe"
    } else {
        "ffmpeg"
    };
    let search_path = env::var_os("PATH").unwrap_or_default();
    let dirs = env::split_paths(&search_path);
    #[cfg(target_os = "macos")]
    let dirs = dirs.chain([
        PathBuf::from("/opt/homebrew/bin"),
        PathBuf::from("/usr/local/bin"),
    ]);
    dirs.filter(|dir| dir.is_absolute())
        .map(|dir| dir.join(name))
        .find(|path| path.is_file())
}

struct Conversion {
    child: Child,
    _directory: TempDir,
    output: PathBuf,
    started: Instant,
}

impl Conversion {
    fn start(source: &str, format: PreviewFormat) -> Result<Self, String> {
        let executable = ffmpeg().ok_or("请安装 FFmpeg 并将它加入 PATH，然后重新启动 Pudding。")?;
        let directory = tempfile::Builder::new()
            .prefix("pudding-video-")
            .tempdir()
            .map_err(|error| format!("无法创建视频预览临时目录：{error}"))?;
        let (name, encoder_options) = format.output();
        let output = directory.path().join(name);
        let mut command = Command::new(executable);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x08000000); // CREATE_NO_WINDOW
        }
        let child = command
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            // Only local media containers can be opened. In particular, FFmpeg
            // must not interpret an input as a playlist of URLs or other files.
            .args([
                "-nostdin", "-v", "error", "-xerror", "-protocol_whitelist", "file",
                "-format_whitelist", "mov,matroska,avi,mpeg,mpegts,mpegvideo,ogg", "-i",
            ])
            .arg(format!("file:{source}"))
            .args([
                "-map", "0:V:0", "-map", "0:a:0?", "-map_metadata", "-1", "-map_chapters", "-1",
                "-vf", "scale=w='min(1280,iw)':h='min(720,ih)':force_original_aspect_ratio=decrease:force_divisible_by=2",
                "-pix_fmt", "yuv420p", "-fpsmax", "30", "-threads:v", "2", "-ac", "2", "-b:a", "128k",
            ])
            .args(encoder_options)
            .arg("-fs")
            .arg(MAX_BYTES.to_string())
            .arg(&output)
            .spawn()
            .map_err(|error| format!("无法启动 FFmpeg：{error}"))?;
        Ok(Self {
            child,
            _directory: directory,
            output,
            started: Instant::now(),
        })
    }

    fn ready(&mut self) -> Result<bool, String> {
        let status = self
            .child
            .try_wait()
            .map_err(|error| format!("无法读取视频转换状态：{error}"))?;
        let Some(status) = status else {
            if self.started.elapsed() >= TIMEOUT {
                return Err("视频转换超时，请使用默认应用打开。".into());
            }
            return Ok(false);
        };
        if !status.success() {
            return Err(
                "FFmpeg 无法转换该视频。文件可能已损坏，或当前 FFmpeg 缺少预览所需的编解码器。"
                    .into(),
            );
        }
        let bytes = fs::metadata(&self.output)
            .map_err(|error| format!("无法读取转换后的视频：{error}"))?
            .len();
        if bytes >= MAX_BYTES {
            return Err("转换后的视频超过 512 MB，请使用默认应用打开。".into());
        }
        if bytes == 0 {
            return Err("FFmpeg 未生成可播放的视频。".into());
        }
        Ok(true)
    }
}

impl Drop for Conversion {
    fn drop(&mut self) {
        if !matches!(self.child.try_wait(), Ok(Some(_))) {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

struct VideoConversion(Mutex<Conversion>);

impl Resource for VideoConversion {}

fn allow_source(manager: &impl Manager<tauri::Wry>, path: &Path) -> Result<String, String> {
    manager
        .asset_protocol_scope()
        .allow_file(path)
        .map_err(|error| format!("无法预览视频：{error}"))?;
    Ok(filesystem::display(path))
}

#[tauri::command]
pub(crate) async fn prepare_video_preview(
    app: tauri::AppHandle,
    path: String,
) -> Result<String, String> {
    let path = blocking(move || filesystem::video_preview_path(&path)).await?;
    allow_source(&app, Path::new(&path))
}

#[tauri::command]
pub(crate) async fn start_video_conversion(
    webview: Webview,
    path: String,
    format: PreviewFormat,
) -> Result<ResourceId, String> {
    let conversion = blocking(move || {
        let source = filesystem::video_preview_path(&path)?;
        Conversion::start(&source, format)
    })
    .await?;
    Ok(webview
        .resources_table()
        .add(VideoConversion(Mutex::new(conversion))))
}

#[tauri::command]
pub(crate) fn video_conversion_source(
    webview: Webview,
    rid: ResourceId,
) -> Result<Option<String>, String> {
    let resource = webview
        .resources_table()
        .get::<VideoConversion>(rid)
        .map_err(|error| error.to_string())?;
    let mut conversion = resource.0.lock().map_err(|_| "无法读取视频转换状态。")?;
    if conversion.ready()? {
        return allow_source(&webview, &conversion.output).map(Some);
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires a local FFmpeg build with libx264 and libvpx"]
    fn converts_video_to_mp4_and_webm_and_removes_temporary_files() {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("示例 #视频.mp4");
        fs::write(
            &source,
            include_bytes!("../../src/lib/demo-videos/sample.mp4"),
        )
        .unwrap();
        let path = filesystem::video_preview_path(&filesystem::display(&source)).unwrap();
        for format in [PreviewFormat::Mp4, PreviewFormat::Webm] {
            let mut conversion = Conversion::start(&path, format).unwrap();
            let output = conversion.output.clone();
            let deadline = Instant::now() + Duration::from_secs(10);
            while !conversion.ready().unwrap() {
                assert!(Instant::now() < deadline, "video conversion timed out");
                std::thread::sleep(Duration::from_millis(20));
            }
            let bytes = fs::read(&output).unwrap();
            match format {
                PreviewFormat::Mp4 => assert_eq!(&bytes[4..8], b"ftyp"),
                PreviewFormat::Webm => assert!(bytes.starts_with(&[0x1a, 0x45, 0xdf, 0xa3])),
            }
            drop(conversion);
            assert!(!output.parent().unwrap().exists());
        }
    }
}
