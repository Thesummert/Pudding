#!/bin/bash
# Compatible with the Bash 3.2 shipped with macOS.
set -Eeuo pipefail

usage() {
  cat <<'EOF'
用法：./scripts/build-macos.sh [选项]
      npm run build:macos -- [选项]

默认安装锁定的 npm 依赖，并构建当前 Mac 架构的 Release .app 和 .dmg。

选项：
  --target <架构>     native（默认）、arm64、intel、universal，或 macOS Rust target triple
  --bundles <格式>    app,dmg（默认）、app、dmg
  --skip-install     使用现有 node_modules，跳过 npm ci
  -h, --help         显示帮助

示例：
  npm run build:macos
  npm run build:macos -- --skip-install --bundles app
  npm run build:macos -- --target universal

通用包需要先安装两个 Rust target：
  rustup target add aarch64-apple-darwin x86_64-apple-darwin

签名和公证沿用 Tauri 配置及当前进程中的 Apple 签名环境变量。
EOF
}

fail() {
  printf '错误：%s\n' "$*" >&2
  exit 1
}

trap 'build_status=$?; printf "macOS 构建失败（第 %s 行，退出码 %s）。\n" "$LINENO" "$build_status" >&2; exit "$build_status"' ERR

target=native
bundles=app,dmg
skip_install=false
while [[ $# -gt 0 ]]; do
  case "$1" in
    --target|--bundles)
      [[ $# -ge 2 && -n "$2" && "$2" != -* ]] || fail "$1 需要指定参数；使用 --help 查看用法。"
      if [[ "$1" == --target ]]; then target=$2; else bundles=$2; fi
      shift 2
      ;;
    --skip-install) skip_install=true; shift ;;
    -h|--help) usage; exit 0 ;;
    *) fail "未知参数：$1；使用 --help 查看用法。" ;;
  esac
done

case "$target" in
  native) ;;
  arm64|aarch64-apple-darwin) target=aarch64-apple-darwin ;;
  intel|x86_64-apple-darwin) target=x86_64-apple-darwin ;;
  universal|universal-apple-darwin) target=universal-apple-darwin ;;
  *) fail "不支持的 macOS 架构：$target" ;;
esac
case "$bundles" in app|dmg|app,dmg) ;; *) fail "--bundles 仅支持 app、dmg 或 app,dmg。" ;; esac

[[ "$(uname -s)" == Darwin ]] || fail '此脚本需要在 macOS 上运行。'
project_root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
cd -- "$project_root"

printf '检查 macOS 构建环境…\n'
for tool in node npm cargo rustc xcode-select xcrun; do
  command -v "$tool" >/dev/null 2>&1 || fail "缺少 ${tool}，请先安装 Node.js、Rust 和 Xcode Command Line Tools。"
done
node -e 'const [major, minor] = process.versions.node.split(".").map(Number); process.exit((major === 20 && minor >= 19) || (major === 22 && minor >= 12) || major > 22 ? 0 : 1)' \
  || fail 'Node.js 需要 20.19+（20.x）或 22.12+；建议安装当前 LTS。'
xcode-select -p >/dev/null 2>&1 && xcrun --find clang >/dev/null 2>&1 && xcrun --sdk macosx --show-sdk-path >/dev/null 2>&1 \
  || fail '无法使用 macOS SDK；请运行 xcode-select --install，并确认开发者工具配置正常。'

if [[ "$target" != native ]]; then
  if [[ "$target" == universal-apple-darwin ]]; then
    required_targets=(aarch64-apple-darwin x86_64-apple-darwin)
  else
    required_targets=("$target")
  fi
  rust_sysroot=$(rustc --print sysroot)
  for rust_target in "${required_targets[@]}"; do
    [[ -d "$rust_sysroot/lib/rustlib/$rust_target/lib" ]] \
      || fail "缺少 Rust target：${rust_target}。请运行 rustup target add $rust_target 后重试。"
  done
fi

if [[ "$skip_install" == true ]]; then
  [[ -x node_modules/.bin/tauri && -x node_modules/.bin/vite && -x node_modules/.bin/svelte-check ]] \
    || fail '前端依赖不完整；请去掉 --skip-install，或先运行 npm ci。'
else
  printf '安装 npm 依赖（npm ci）…\n'
  npm ci
fi

build_args=(--ci --bundles "$bundles")
if [[ "$target" != native ]]; then build_args+=(--target "$target"); fi
printf '构建 macOS Release：架构=%s，格式=%s\n' "$target" "$bundles"
# Tauri runs beforeBuildCommand, including Svelte checks and the frontend build.
# Arguments after the final -- are passed to Cargo; keep Cargo.lock unchanged.
npm run tauri -- build "${build_args[@]}" -- --locked

# Cargo metadata also handles CARGO_TARGET_DIR and Cargo configuration overrides.
target_directory=$(cargo metadata --manifest-path src-tauri/Cargo.toml --no-deps --format-version 1 --locked --offline \
  | node -e 'let input = ""; process.stdin.setEncoding("utf8"); process.stdin.on("data", chunk => input += chunk); process.stdin.on("end", () => process.stdout.write(JSON.parse(input).target_directory));')
if [[ "$target" == native ]]; then
  bundle_directory="$target_directory/release/bundle"
else
  bundle_directory="$target_directory/$target/release/bundle"
fi

printf '\n构建完成，产物：\n'
shopt -s nullglob
if [[ "$bundles" == *app* ]]; then
  app_artifacts=("$bundle_directory"/macos/*.app)
  [[ ${#app_artifacts[@]} -gt 0 ]] || fail "未找到 .app：$bundle_directory/macos"
  printf '  %s\n' "${app_artifacts[@]}"
fi
if [[ "$bundles" == *dmg* ]]; then
  dmg_artifacts=("$bundle_directory"/dmg/*.dmg)
  [[ ${#dmg_artifacts[@]} -gt 0 ]] || fail "未找到 .dmg：$bundle_directory/dmg"
  printf '  %s\n' "${dmg_artifacts[@]}"
fi
