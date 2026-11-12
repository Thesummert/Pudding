#!/usr/bin/env bash
set -Eeuo pipefail
usage() {
  cat <<'HELP'
用法：npm run build:arch -- [--skip-install]
      ./scripts/build-arch.sh [--skip-install]

在 Arch Linux 上构建当前架构的 pacman 安装包（.pkg.tar.zst）。
默认运行 npm ci；使用 --skip-install 跳过 npm 依赖安装。
需要预先安装构建依赖及 makepkg；请勿以 root 身份运行。
HELP
}
fail() { printf '错误：%s\n' "$*" >&2; exit 1; }
trap 'status=$?; printf "Arch 构建失败（第 %s 行，退出码 %s）。\n" "$LINENO" "$status" >&2' ERR
skip_install=false
while [[ $# -gt 0 ]]; do
  case "$1" in
    --skip-install) skip_install=true; shift ;;
    -h|--help) usage; exit 0 ;;
    *) fail "未知参数：$1；使用 --help 查看用法。" ;;
  esac
done
[[ "$(uname -s)" == Linux ]] || fail '此脚本需要在 Arch Linux 上运行。'
[[ $(id -u) -ne 0 ]] || fail 'makepkg 不允许以 root 身份运行。'
cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.."
for tool in node npm cargo rustc pkg-config cc makepkg pacman; do
  command -v "$tool" >/dev/null 2>&1 || fail "缺少 $tool，请先安装 README 中列出的依赖。"
done
node -e 'const [major, minor] = process.versions.node.split(".").map(Number); process.exit((major === 20 && minor >= 19) || (major === 22 && minor >= 12) || major > 22 ? 0 : 1)' || fail 'Node.js 需要 20.19+（20.x）或 22.12+。'
for package in gtk+-3.0 webkit2gtk-4.1 openssl; do
  pkg-config --exists "$package" || fail "缺少开发依赖 $package；请参阅 README。"
done
if [[ "$skip_install" == true ]]; then
  [[ -x node_modules/.bin/tauri && -x node_modules/.bin/vite && -x node_modules/.bin/svelte-check ]] || fail '前端依赖不完整；请去掉 --skip-install 或运行 npm ci。'
else
  npm ci
fi
printf '构建 Arch Linux Release…\n'
npm run tauri -- build --ci --no-bundle -- --locked
target_directory=$(cargo metadata --manifest-path src-tauri/Cargo.toml --no-deps --format-version 1 --locked --offline | node -e 'let s=""; process.stdin.on("data", c => s += c); process.stdin.on("end", () => process.stdout.write(JSON.parse(s).target_directory));')
version=$(node -p 'require("./package.json").version')
[[ "$version" =~ ^[0-9]+(\.[0-9]+)*$ ]] || fail "不支持的包版本：$version"
[[ -f "$target_directory/release/pudding" ]] || fail '未找到编译后的 pudding 可执行文件。'
package_dir="$target_directory/release/bundle/pacman"
mkdir -p "$package_dir"
package_dir=$(cd -- "$package_dir" && pwd)
work_dir=$(mktemp -d)
trap 'rm -rf -- "$work_dir"' EXIT
install -m 755 "$target_directory/release/pudding" "$work_dir/pudding"
install -m 644 src-tauri/icons/128x128.png "$work_dir/pudding.png"
cat > "$work_dir/pudding.desktop" <<'DESKTOP'
[Desktop Entry]
Type=Application
Name=Pudding
Comment=轻巧、有序的跨平台文件管理器
Exec=pudding %F
Icon=pudding
Terminal=false
Categories=Utility;FileManager;
DESKTOP
printf 'pkgver=%s\n' "$version" > "$work_dir/PKGBUILD"
cat >> "$work_dir/PKGBUILD" <<'PKGBUILD'
pkgname=pudding
pkgrel=1
pkgdesc='轻巧、有序的跨平台文件管理器'
arch=('x86_64' 'aarch64')
license=('GPL-3.0-or-later')
depends=('gtk3' 'webkit2gtk-4.1' 'openssl' 'xdotool' 'gvfs-smb')
source=('pudding' 'pudding.desktop' 'pudding.png')
sha256sums=('SKIP' 'SKIP' 'SKIP')
package() {
  install -Dm755 "$srcdir/pudding" "$pkgdir/usr/bin/pudding"
  install -Dm644 "$srcdir/pudding.desktop" "$pkgdir/usr/share/applications/pudding.desktop"
  install -Dm644 "$srcdir/pudding.png" "$pkgdir/usr/share/icons/hicolor/128x128/apps/pudding.png"
}
PKGBUILD
(cd "$work_dir" && PKGDEST="$package_dir" makepkg --force --noconfirm)
printf '\n构建完成，安装包位于：%s\n' "$package_dir"
