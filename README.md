# Pudding

使用 **Tauri 2 + Svelte 5 + TypeScript + Rust** 编写的跨平台文件管理器。布局参考 Dolphin：位置侧栏、可编辑路径导航、文件浏览区域和可收起的信息面板。

## 当前功能

- 读取真实本地目录；自动发现主文件夹、可用的常用目录和文件系统入口。
- 后退、前进、上一级、路径面包屑、手动输入绝对路径或 `~/…`、刷新；支持鼠标侧键后退和前进。
- 图标和详细列表两种视图；文件夹优先排序；按名称、修改时间、大小或类型排序。
- 自动刷新已挂载设备；可在侧栏安全推出系统支持的可移除磁盘。
- 安装支持联动的新版 Sugar Archive 后，右键菜单“解压缩”中可快速压缩为 ZIP、在 Sugar 中自定义压缩，或将压缩包解压到当前文件夹或同名文件夹；在文件区空白处右键可压缩当前文件夹。需要重新构建并安装带 `sugar-archive-pudding` 辅助程序的 Sugar，旧版不会显示这些操作。文件名冲突不会覆盖；解压失败可能留下已解压文件。
- 文件右键菜单可显示系统推荐的应用，也可手动选择其他应用打开文件；Linux 选择应用的可执行文件，macOS 选择 `.app`，Windows 选择 `.exe`。
- macOS 和 Linux 可通过侧栏连接 SMB，输入服务器地址和账户后可在应用内选择共享，挂载后自动打开；成功连接的地址保存在最近连接中，密码不会保存。
- 桌面版递归搜索当前文件夹及其子文件夹，默认匹配文件名；可勾选「搜索内容」匹配文本文件，状态栏显示扫描数量。支持显示或隐藏隐藏文件。
- 搜索时按需建立内存索引；监听当前文件夹及子文件夹的外部变更，自动刷新列表并使索引失效。
- 多标签页浏览，每个标签保留独立的路径与导航历史；右键文件夹可在新标签页或新窗口中打开。
- 右键文件夹或文件列表空白处可在系统文件管理器或终端中打开对应目录，并可在设置中选择常用终端或本机的终端应用。
- macOS 应用可接收系统传入的文件夹；也可通过命令行传入文件夹绝对路径。
- 单选、Ctrl / Command 多选、Shift 连续选择、鼠标框选、键盘导航和全选。
- 新建空文件或文件夹、重命名、递归复制、同磁盘剪切粘贴、右键复制文件或文件夹地址（多选时每行一个）、移入系统回收站。
- 系统文件剪贴板：与系统文件管理器互相复制粘贴，支持多项目；切回窗口时同步状态，粘贴时重新读取，复制路径使用原生文本剪贴板。
- 系统拖放：从外部拖入文件或文件夹，拖到当前目录空白、子文件夹、侧栏位置或路径面包屑；按住选中项目拖出到系统文件管理器或其他应用，支持多选。拖入按复制处理，并高亮目标位置；中间浏览区域的空白处复制到当前打开的目录，落在文件夹上则复制到该文件夹内。
- 使用系统默认应用打开文件；查看类型、大小、修改时间及路径。
- 图标网格、详细列表和信息面板显示图片缩略预览；双击、空格键或右键菜单打开大图，支持左右切换、适应窗口和实际大小，并可用默认应用打开。文件缩略图按可见范围加载，读取或解码失败时保留文件图标。
- 图标网格和详细列表支持视频首帧缩略图，按可见范围加载；无法解码时保留视频图标。
- 选中文本、视频、图片或 PDF 后按空格打开预览，再按空格关闭；PDF 支持翻页，视频自动播放并支持播放控制。
- 大图预览支持鼠标滚轮缩放（10%–1000%）和按住左键拖拽；点击预览窗口外侧或按 Esc 退出，切图或切换显示模式时重置缩放与位置。
- 浅色 / 深色外观、图标缩放、信息面板和显示偏好的本地保存。
- 权限错误、名称冲突和空目录提示；后台线程处理文件系统操作。

**浏览器演示与桌面版：** `npm run dev` 在浏览器中显示带「演示模式」标记的示例文件系统，操作仅影响内存数据，刷新页面后重置；搜索仅筛选当前目录的文件名，不支持内容搜索或文件系统监听。`npm run tauri dev` 才会访问真实文件系统。桌面调用出错时不会自动切换到演示数据。

## 本地运行

需要 Node.js **22.12+**（或 20.19+）、npm、Rust stable，以及对应系统的 [Tauri 原生开发依赖](https://v2.tauri.app/start/prerequisites/)。Rust 最低版本为 1.91，建议使用当前 stable。

- macOS：安装 Xcode Command Line Tools：`xcode-select --install`。
- Windows：安装 Visual Studio C++ Build Tools、Windows SDK 和 Microsoft Edge WebView2。
- Linux：安装 WebKitGTK 4.1、GTK 3、OpenSSL 及构建工具；具体包名见上述官方文档。

```sh
npm install
npm run tauri dev
```

浏览器中预览界面：

```sh
npm run dev
# http://127.0.0.1:1420
```

构建本机的桌面安装包：

```sh
npm run tauri build
```

产物位于 `src-tauri/target/release/bundle/`。各操作系统的安装包应在对应系统上构建；发布到其他电脑前还需要根据平台设置签名与公证。

### Linux 和 Arch Linux 构建脚本

在目标 Linux 系统及架构上安装 Node.js（20.19+ 或 22.12+）、Rust（1.91+）、C 编译器、pkg-config、GTK 3、WebKitGTK 4.1、OpenSSL 和相关开发依赖。Debian/Ubuntu 可安装 `build-essential curl wget file libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev libwebkit2gtk-4.1-dev gvfs-backends gvfs-fuse`；Arch Linux 可安装 `base-devel curl wget file xdotool openssl libayatana-appindicator librsvg webkit2gtk-4.1 gvfs-smb nodejs npm rust`，并另行安装 Node.js、npm 和 Rust。AppImage 打包可能需要联网下载工具及 FUSE 支持。

```bash
# 通用 Linux：默认生成 deb
npm run build:linux
npm run build:linux -- --skip-install --bundles rpm,appimage

# Arch Linux：生成 pacman 安装包（不要以 root 身份运行）
npm run build:arch
npm run build:arch -- --skip-install
```

脚本默认运行 `npm ci`，Tauri 会执行前端检查和构建，Rust 使用锁定依赖。Linux 脚本产物位于 Cargo target 目录下的 `release/bundle/`；可通过 `--bundles` 选择 `deb`、`rpm`、`appimage` 或逗号分隔的组合。打包格式还可能需要相应发行版的打包工具。Arch 脚本使用 makepkg 生成 pacman 包，位于 `release/bundle/pacman/`，可用 `sudo pacman -U 路径/包名.pkg.tar.zst` 安装。包扩展名取决于 makepkg 的压缩配置。

### macOS 构建脚本

在 Mac 上执行以下命令，脚本会检查 Node.js、Rust 和 macOS SDK，运行 `npm ci`，再构建当前架构的 Release `.app` 和 `.dmg`。前端检查与打包由 Tauri 自动执行，Rust 构建使用 `--locked`。

```sh
npm run build:macos

# 已安装依赖时，仅构建 .app
npm run build:macos -- --skip-install --bundles app

# 构建兼容 Apple Silicon 和 Intel 的通用包
rustup target add aarch64-apple-darwin x86_64-apple-darwin
npm run build:macos -- --target universal

# 查看全部选项；使用脚本的绝对路径可从任意目录运行
./scripts/build-macos.sh --help
```

`--target` 支持 `native`（默认）、`arm64`、`intel`、`universal` 及对应的 Rust target triple。指定架构前需要安装相应 Rust target；脚本会在缺失时提示安装命令。`--bundles` 支持 `app`、`dmg` 或 `app,dmg`（默认）。

默认产物位于 `src-tauri/target/release/bundle/macos/` 和 `src-tauri/target/release/bundle/dmg/`；指定架构后位于 `src-tauri/target/<target>/release/bundle/`。脚本也支持 `CARGO_TARGET_DIR`，并在成功后打印实际产物路径。签名和公证沿用 Tauri 配置及传入的 Apple 签名环境变量；脚本不会创建证书或配置 Apple 账号。

## 验证

```sh
npm run check
npm test
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
npm run test:e2e
```

本地端到端测试默认使用已安装的 Google Chrome。CI 使用 Playwright Chromium，可通过 `npx playwright install --with-deps chromium` 安装后设置 `CI=1` 运行。测试会验证导航历史、地址输入、隐藏文件、筛选、偏好保存、新建、重命名冲突、复制、剪切、回收站以及图片解码、预览切换和键盘交互。Rust 测试仅操作临时目录，不会删除实际用户文件或向系统回收站写入内容。

## 快捷键

macOS 使用 Command，其他系统使用 Ctrl。

| 快捷键 | 操作 |
| --- | --- |
| Ctrl / Command + T / W | 新建 / 关闭标签页 |
| Ctrl / Command + L | 编辑路径 |
| Ctrl / Command + F | 聚焦搜索框 |
| Alt + ← / → / ↑ | 后退 / 前进 / 上一级 |
| Ctrl / Command + Shift + N | 新建文件夹 |
| Ctrl / Command + A | 全选 |
| Ctrl / Command + C / X / V | 复制 / 剪切 / 粘贴 |
| Ctrl / Command + Shift + . | 切换隐藏文件 |
| F2 | 重命名 |
| Delete | 移入回收站，需确认 |
| Shift + Delete | 永久删除，需确认 |
| Enter | 打开选中项目 |
| Space | 预览选中的文本、视频、图片或 PDF，再次按下关闭 |
| ← / →、Esc（图片预览中） | 上一张 / 下一张、关闭预览 |
| F5 | 刷新 |

## 项目结构

```text
src/
  App.svelte             界面、导航和交互状态
  app.css                浅色 / 深色样式
  lib/
    api.ts               类型化 Tauri 命令入口
    demo.ts              独立的浏览器演示文件系统
    files.ts             排序、格式化、路径和名称规则
    types.ts             前后端数据协议
    FileIcon.svelte      文件与文件夹图标
    FileThumbnail.svelte 按可见范围加载图片和视频缩略图
    NameDialog.svelte    新建与重命名对话框
    ImagePreview.svelte  图片加载、错误提示与资源释放
    ImagePreviewDialog.svelte  大图预览与切换
src-tauri/
  src/filesystem.rs      文件系统逻辑与 Rust 测试
  src/search.rs          递归索引、搜索和文件系统监听
  src/clipboard.rs       系统文件/文本剪贴板与拖出路径校验
  src/rename.rs          各平台原子防覆盖重命名
  src/lib.rs             Tauri 命令及后台任务
  capabilities/          窗口权限
tests/                   单元与端到端测试
```

## 第一阶段的边界

- 桌面搜索不会递归进入符号链接指向的目录，也不会读取符号链接的内容；无法读取的子目录或文件会被跳过。内容搜索仅支持不超过 2 MB、不含 NUL 字节的 UTF-8 文本文件，不解析 PDF、Office 文档等二进制格式。索引保存在内存中，外部变更后失效并在下次搜索时重建；扫描期间状态栏显示已扫描项目数，不提供百分比。
- 每个窗口监听当前活动标签页的目录，切换标签时重新监听并读取目录；标签页和窗口状态不会跨应用重启保存。文件系统监听依赖操作系统支持，监听失败时界面会提示。
- 桌面版使用系统文件剪贴板；Windows 使用文件列表及 Preferred DropEffect，Linux 使用 URI 列表及 GNOME 复制/剪切格式。macOS 系统文件格式没有通用剪切标记，Pudding 的剪切标记用于 Pudding 内粘贴，其他应用可读取文件列表并按其自身规则处理。浏览器演示只支持会话内剪贴操作与内部拖放。
- 拖入时可选择复制或移动，遇到同名项目可选择替换、并存或取消；拖出提供原生文件列表与复制操作，最终行为由接收应用处理。Linux 剪贴板目前使用 X11（Wayland 会话需要 XWayland），原生 Wayland 拖放兼容性尚未验证。
- 设备列表显示已由系统挂载的卷；Linux 使用 `gio mount --unmount`，Windows 仅对系统标记为可移动或光驱的盘符提供推出操作。实际推出能力由系统和设备决定。
- 复制或移动遇到同名目标时，可为整批冲突选择替换、并存或取消；替换文件夹会完整替换原文件夹，不会合并内容。同一项目不能替换自身，可选择并存。复制符号链接和特殊文件暂不支持；目录中的符号链接会导致该项复制失败并尝试清理目标。符号链接仍可浏览、重命名或移入回收站。
- 剪切使用文件系统重命名，不支持跨磁盘移动；可先复制，确认成功后再将源项目移入回收站。
- 多项目操作可能部分成功，失败提示会说明已完成数量，界面会重新读取目录；暂未实现事务回滚、撤销和精确进度。
- 图片预览支持 PNG、JPEG、GIF、WebP、SVG、AVIF、HEIC 和 BMP，实际解码能力取决于系统 WebView；不支持的格式或损坏文件在缩略图中回退为文件图标，信息面板和大图预览会显示错误提示。单张图片限制为 32 MB，较大的图片可使用默认应用打开。视频预览支持 MP4、M4V、MOV、MKV、AVI、WebM、MPEG 和 OGV，实际解码能力取决于系统 WebView。文本预览限 2 MB，PDF 预览限 64 MB；不支持的编码和过大的文件可使用默认应用打开。暂不提供其他文件内容预览或目录递归大小。
- macOS 和 Linux 使用 SMB2/3 枚举共享并由系统挂载；Linux 需要 GVfs、SMB 后端和 FUSE（Arch 安装 `gvfs-smb`，Debian/Ubuntu 安装 `gvfs-backends` 和 `gvfs-fuse`），并需有正常运行的桌面用户会话。Windows 可浏览已由系统挂载且可通过本地路径访问的共享。双栏浏览和回收站浏览尚未实现。

桌面应用使用操作系统当前用户权限访问文件。渲染进程只加载本地内容，通过明确的 Rust 命令调用文件操作，CSP 禁止远程页面和脚本。

## 原生交互验证说明

此前的剪贴板与拖放改动已在 macOS 环境通过 Svelte / TypeScript 检查、前端生产构建、前端单元与 DOM 集成测试、Chrome 端到端测试及 Rust 单元测试和 Clippy。测试覆盖粘贴时读取外部更新、剪贴板写入失败、剪切部分失败后保留新剪贴板、原生拖入事件的平台坐标换算、Retina 屏幕下中间区域不误命中侧栏与目标路由、监听器释放，以及图标/列表向文件夹和侧栏拖放。当时的前端、端到端和 Rust 测试以及 1 项 macOS 原生剪贴板测试已通过。原生测试使用独立命名的剪贴板，验证多文件、中文/特殊字符、目录 URL 和剪切标记，不修改用户当前系统剪贴板。由于需要访问系统剪贴板服务，该测试默认忽略，可在 macOS 桌面会话中单独运行：

```sh
cargo test --manifest-path src-tauri/Cargo.toml macos_file_items -- --ignored
```

跨应用文件剪贴板和拖出到 Finder 等原生交互仍需桌面实机手动验证；Windows / Linux 尚未实机验证。
