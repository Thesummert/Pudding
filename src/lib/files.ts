import type { FileEntry, SortKey } from './types';

const collator = new Intl.Collator('zh-CN', { numeric: true, sensitivity: 'base' });

const imageTypes: Record<string, string> = { png: 'image/png', jpg: 'image/jpeg', jpeg: 'image/jpeg', gif: 'image/gif', webp: 'image/webp', svg: 'image/svg+xml', avif: 'image/avif', heic: 'image/heic', bmp: 'image/bmp' };

export function imageMimeType(extension: string): string | undefined {
  return Object.hasOwn(imageTypes, extension.toLowerCase()) ? imageTypes[extension.toLowerCase()] : undefined;
}

export function isImage(entry: FileEntry): boolean {
  return !entry.isDir && imageMimeType(entry.extension) !== undefined;
}

export function isVideo(entry: FileEntry): boolean {
  return !entry.isDir && ['mp4', 'm4v', 'mov', 'mkv', 'avi', 'webm', 'mpeg', 'mpg', 'ogv'].includes(entry.extension.toLowerCase());
}

const textExtensions = new Set(['txt', 'md', 'markdown', 'csv', 'tsv', 'log', 'json', 'jsonl', 'xml', 'yaml', 'yml', 'toml', 'ini', 'conf', 'cfg', 'html', 'htm', 'css', 'scss', 'js', 'jsx', 'ts', 'tsx', 'svelte', 'vue', 'rs', 'py', 'sh', 'bash', 'zsh', 'c', 'h', 'cpp', 'hpp', 'java', 'go', 'swift', 'sql', 'rb', 'php', 'properties']);
const textNames = new Set(['readme', 'license', 'licence', 'makefile', 'dockerfile', '.gitignore', '.gitattributes', '.editorconfig', '.env', '.bashrc', '.zshrc']);

export function isText(entry: FileEntry): boolean {
  return !entry.isDir && (textExtensions.has(entry.extension.toLowerCase()) || textNames.has(entry.name.toLowerCase()));
}

export function isPdf(entry: FileEntry): boolean {
  return !entry.isDir && entry.extension.toLowerCase() === 'pdf';
}

export function canPreview(entry: FileEntry): boolean {
  return isImage(entry) || isVideo(entry) || isText(entry) || isPdf(entry);
}

export function formatSize(bytes: number): string {
  if (!bytes) return '0 B';
  const unit = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), 4);
  return `${new Intl.NumberFormat('zh-CN', { maximumFractionDigits: unit > 0 ? 1 : 0 }).format(bytes / 1024 ** unit)} ${['B', 'KB', 'MB', 'GB', 'TB'][unit]}`;
}

export function formatDate(value: number | null): string {
  if (value === null) return '未知';
  return new Intl.DateTimeFormat('zh-CN', { year: 'numeric', month: '2-digit', day: '2-digit', hour: '2-digit', minute: '2-digit', hour12: false }).format(value * 1000);
}

export function fileKind(entry: FileEntry): string {
  if (entry.isSymlink) return entry.isDir ? '文件夹链接' : '符号链接';
  if (entry.isDir) return '文件夹';
  const ext = entry.extension.toLowerCase();
  if (isImage(entry)) return '图像';
  if (isVideo(entry)) return '视频';
  if (['mp3', 'wav', 'flac', 'ogg', 'm4a', 'aac'].includes(ext)) return '音频';
  if (['zip', 'rar', '7z', 'gz', 'tar', 'xz'].includes(ext)) return '压缩文件';
  if (ext === 'pdf') return 'PDF 文档';
  if (['ts', 'js', 'rs', 'py', 'json', 'html', 'css', 'svelte', 'toml', 'yaml', 'yml', 'sh'].includes(ext)) return '代码文件';
  if (['md', 'txt', 'doc', 'docx', 'odt', 'rtf'].includes(ext)) return '文档';
  return ext ? `${ext.toUpperCase()} 文件` : '文件';
}

export function visibleEntries(entries: FileEntry[], query: string, hidden: boolean, key: SortKey, descending: boolean): FileEntry[] {
  const term = query.trim().toLocaleLowerCase();
  return entries
    .filter((entry) => (hidden || !entry.hidden) && (!term || entry.name.toLocaleLowerCase().includes(term)))
    .sort((a, b) => {
      if (a.isDir !== b.isDir) return a.isDir ? -1 : 1;
      let order = 0;
      if (key === 'name') order = collator.compare(a.name, b.name);
      if (key === 'modified') order = (a.modified ?? 0) - (b.modified ?? 0);
      if (key === 'size') order = a.size - b.size;
      if (key === 'type') order = collator.compare(fileKind(a), fileKind(b));
      return (order || collator.compare(a.name, b.name)) * (descending ? -1 : 1);
    });
}

export function breadcrumbs(path: string): { name: string; path: string }[] {
  const normalized = path.replace(/\\/g, '/');
  const parts = normalized.split('/').filter(Boolean);
  const windows = /^[A-Za-z]:/.test(normalized);
  const unc = normalized.startsWith('//');
  const prefix = unc ? '//' : windows ? '' : '/';
  return [
    ...(!windows && !unc ? [{ name: '/', path: '/' }] : []),
    ...parts.map((name, i) => ({ name, path: `${prefix}${parts.slice(0, i + 1).join('/')}${windows && i === 0 ? '/' : ''}` })),
  ];
}

export function basename(path: string): string {
  return path.replace(/[\\/]+$/, '').split(/[\\/]/).pop() || '/';
}

export function validateName(name: string): string | null {
  if (!name.trim()) return '请输入名称。';
  if (name === '.' || name === '..' || /[<>:"/\\|?*\u0000-\u001f]/.test(name)) return '名称不能包含路径分隔符或特殊字符。';
  if (/[. ]$/.test(name)) return '名称不能以空格或句点结尾。';
  if (/^(con|prn|aux|nul|com[1-9]|lpt[1-9])(\.|$)/i.test(name)) return '此名称是系统保留名称。';
  return null;
}
