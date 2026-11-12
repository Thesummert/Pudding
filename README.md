# Pudding

使用 **Tauri 2 + Svelte 5 + TypeScript + Rust** 编写的跨平台文件管理器。布局参考 Dolphin：位置侧栏、可编辑路径导航、文件浏览区域和可收起的信息面板。

## 当前功能

- 读取真实本地目录；自动发现主文件夹、可用的常用目录和文件系统入口。
- 后退、前进、上一级、路径面包屑、手动输入绝对路径或 `~/…`、刷新。
- 图标和详细列表两种视图；文件夹优先排序；按名称、修改时间、大小或类型排序。
- 当前目录内按名称筛选；显示或隐藏隐藏文件。
- 单选、Ctrl / Command 多选、Shift 连续选择、键盘导航和全选。
- 新建文件夹、重命名、递归复制、同磁盘剪切粘贴、复制路径、移入系统回收站。
- 使用系统默认应用打开文件；查看类型、大小、修改时间及路径。
- 浅色 / 深色外观、图标缩放、信息面板和显示偏好的本地保存。
- 权限错误、名称冲突和空目录提示；后台线程处理文件系统操作。

**浏览器演示与桌面版：** `npm run dev` 在浏览器中显示带「演示模式」标记的示例文件系统，操作仅影响内存数据，刷新页面后重置。`npm run tauri dev` 才会访问真实文件系统。桌面调用出错时不会自动切换到演示数据。

## 本地运行

需要 Node.js **22.12+**（或 20.19+）、npm、Rust stable，以及对应系统的 [Tauri 原生开发依赖](https://v2.tauri.app/start/prerequisites/)。Rust 最低版本为 1.90，建议使用当前 stable。

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

## 验证

```sh
npm run check
npm test
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
npm run test:e2e
```

本地端到端测试默认使用已安装的 Google Chrome。CI 使用 Playwright Chromium，可通过 `npx playwright install --with-deps chromium` 安装后设置 `CI=1` 运行。测试会验证导航历史、地址输入、隐藏文件、筛选、偏好保存、新建、重命名冲突、复制、剪切和回收站交互。Rust 测试仅操作临时目录，不会删除实际用户文件或向系统回收站写入内容。

## 快捷键

macOS 使用 Command，其他系统使用 Ctrl。

| 快捷键 | 操作 |
| --- | --- |
| Ctrl / Command + L | 编辑路径 |
| Ctrl / Command + F | 搜索当前文件夹 |
| Alt + ← / → / ↑ | 后退 / 前进 / 上一级 |
| Ctrl / Command + Shift + N | 新建文件夹 |
| Ctrl / Command + A | 全选 |
| Ctrl / Command + C / X / V | 复制 / 剪切 / 粘贴 |
| Ctrl / Command + Shift + . | 切换隐藏文件 |
| F2 | 重命名 |
| Delete | 移入回收站，需确认 |
| Enter | 打开选中项目 |
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
    NameDialog.svelte    新建与重命名对话框
src-tauri/
  src/filesystem.rs      文件系统逻辑与 Rust 测试
  src/rename.rs          各平台原子防覆盖重命名
  src/lib.rs             Tauri 命令及后台任务
  capabilities/          窗口权限
tests/                   单元与端到端测试
```

## 第一阶段的边界

- 搜索只筛选当前目录名称，暂未实现递归全文搜索、文件索引和文件系统实时监听；外部变更后请刷新。
- 剪贴板保存于当前应用会话，暂未接入系统文件剪贴板或系统拖放。
- 同名目标会报错，不会询问覆盖。复制符号链接和特殊文件暂不支持；目录中的符号链接会导致该项复制失败并尝试清理目标。符号链接仍可浏览、重命名或移入回收站。
- 剪切使用文件系统重命名，不支持跨磁盘移动；可先复制，确认成功后再将源项目移入回收站。
- 多项目操作可能部分成功，失败提示会说明已完成数量，界面会重新读取目录；暂未实现事务回滚、撤销和精确进度。
- 信息面板显示元数据，暂不生成图片缩略图或文件内容预览，也不计算目录递归大小。
- 设备区域展示系统根目录、Windows 可访问盘符，以及 macOS `/Volumes` 或 Linux `/media`、`/mnt` 下的目录入口；暂未实现设备热插拔监控或卸载。
- 多标签页、双栏浏览、远程协议和回收站浏览留待后续阶段。

桌面应用使用操作系统当前用户权限访问文件。渲染进程只加载本地内容，通过明确的 Rust 命令调用文件操作，CSP 禁止远程页面和脚本。

## 本次验证范围

已在 macOS 环境通过 Svelte / TypeScript 检查（0 错误、0 警告）、前端单元与 DOM 集成测试、Rust 单元测试、Clippy 和 macOS 应用包构建。DOM 集成测试直接挂载实际 Svelte 界面，验证导航、筛选、偏好保存和文件操作流程。

当前执行沙箱禁止监听本地端口，且 Chrome 在启动阶段中止，因此 Playwright 浏览器测试尚未完成。Windows / Linux 已配置 CI 检查，但尚未在这两个系统上实机验证。
