#!/usr/bin/env bash
set -Eeuo pipefail

usage() {
  cat <<'HELP'
用法：./scripts/build-linux.sh [选项]
      npm run build:linux -- [选项]

在当前 Linux 架构构建 Release 安装包，默认格式为 deb。
  --bundles <格式>  deb、rpm、appimage 或以逗号分隔的组合
  --skip-install   跳过 npm ci，使用现有 node_modules
  -h, --help       显示帮助

Arch Linux 可使用 npm run build:arch 构建 pacman 安装包。
HELP
}
fail() { printf '错误：%s\n' "$*" >&2; exit 1; }
trap 'status=$?; printf "Linux 构建失败（第 %s 行，退出码 %s）。\n" "$LINENO" "$status" >&2' ERR
bundles=deb
skip_install=false
while [[ $# -gt 0 ]]; do
  case "$1" in
    --bundles) [[ $# -ge 2 && -n "$2" && "$2" != -* ]] || fail '--bundles 需要指定格式。'; bundles=$2; shift 2 ;;
    --skip-install) skip_install=true; shift ;;
    -h|--help) usage; exit 0 ;;
    *) fail "未知参数：$1；使用 --help 查看用法。" ;;
  esac
done
[[ "$bundles" =~ ^(deb|rpm|appimage)(,(deb|rpm|appimage))*$ ]] || fail '格式须为 deb、rpm、appimage 或其逗号分隔组合。'
[[ "$(uname -s)" == Linux ]] || fail '此脚本需要在 Linux 上运行。'
cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.."
for tool in node npm cargo rustc pkg-config cc; do
  command -v "$tool" >/dev/null 2>&1 || fail "缺少 $tool，请先安装构建依赖。"
done
node -e 'const [major, minor] = process.versions.node.split(".").map(Number); process.exit((major === 20 && minor >= 19) || (major === 22 && minor >= 12) || major > 22 ? 0 : 1)' || fail 'Node.js 需要 20.19+（20.x）或 22.12+。'
for package in gtk+-3.0 webkit2gtk-4.1 openssl; do
  pkg-config --exists "$package" || fail "缺少开发依赖 $package；请参阅 README 中的安装说明。"
done
if [[ "$skip_install" == true ]]; then
  [[ -x node_modules/.bin/tauri && -x node_modules/.bin/vite && -x node_modules/.bin/svelte-check ]] || fail '前端依赖不完整；请去掉 --skip-install 或运行 npm ci。'
else
  npm ci
fi
printf '构建 Linux Release：格式=%s\n' "$bundles"
npm run tauri -- build --ci --bundles "$bundles" -- --locked
target_directory=$(cargo metadata --manifest-path src-tauri/Cargo.toml --no-deps --format-version 1 --locked --offline | node -e 'let s=""; process.stdin.on("data", c => s += c); process.stdin.on("end", () => process.stdout.write(JSON.parse(s).target_directory));')
printf '\n构建完成，产物目录：%s/release/bundle\n' "$target_directory"
