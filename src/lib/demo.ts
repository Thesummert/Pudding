import type { AppInfo, DirectoryListing, FileEntry } from './types';
import { basename, isImage, validateName } from './files';
import coast from './demo-images/coast.svg?raw';
import mountains from './demo-images/mountains.svg?raw';

const home = '/home/pudding';
const now = Math.floor(Date.now() / 1000);
const folders: Record<string, FileEntry[]> = {};
const imageContents: Record<string, string> = { [`${home}/Pictures/海岸.svg`]: coast, [`${home}/Pictures/山间.svg`]: mountains };

function entry(parent: string, name: string, isDir: boolean, size = 0, hours = 0): FileEntry {
  return { name, path: `${parent}/${name}`, isDir, isSymlink: false, size, modified: now - hours * 3600, hidden: name.startsWith('.'), extension: isDir ? '' : name.split('.').slice(1).pop() ?? '' };
}

const places = [
  { id: 'home', name: '主文件夹', path: home },
  ...[['desktop', '桌面', 'Desktop'], ['documents', '文档', 'Documents'], ['downloads', '下载', 'Downloads'], ['pictures', '图片', 'Pictures'], ['music', '音乐', 'Music'], ['videos', '视频', 'Videos']].map(([id, name, folder]) => ({ id, name, path: `${home}/${folder}` })),
];

folders[home] = [
  ...['Desktop', 'Documents', 'Downloads', 'Music', 'Pictures', 'Videos', 'Projects', 'Archive'].map((name, i) => entry(home, name, true, 0, i * 17)),
  entry(home, '开始使用.md', false, 2480, 1),
  entry(home, '灵感笔记.txt', false, 1280, 24),
  entry(home, '.config', true, 0, 12),
];
folders['/'] = [entry('', 'home', true)];
folders['/home'] = [entry('/home', 'pudding', true)];
for (const folder of folders[home].filter((f) => f.isDir)) folders[folder.path] = [];
folders[`${home}/Documents`] = [entry(`${home}/Documents`, '工作资料', true), entry(`${home}/Documents`, '项目计划.md', false, 8360, 3), entry(`${home}/Documents`, '阅读清单.pdf', false, 2841600, 48)];
folders[`${home}/Documents/工作资料`] = [];
folders[`${home}/Downloads`] = [entry(`${home}/Downloads`, '设计素材.zip', false, 18403200, 6), entry(`${home}/Downloads`, '使用说明.pdf', false, 432580, 24)];
folders[`${home}/Pictures`] = [entry(`${home}/Pictures`, '旅行', true), entry(`${home}/Pictures`, '壁纸', true), entry(`${home}/Pictures`, '海岸.svg', false, coast.length), entry(`${home}/Pictures`, '山间.svg', false, mountains.length)];
folders[`${home}/Pictures/旅行`] = [];
folders[`${home}/Pictures/壁纸`] = [];
folders[`${home}/Projects`] = [entry(`${home}/Projects`, 'pudding', true), entry(`${home}/Projects`, 'README.md', false, 3120, 2)];
folders[`${home}/Projects/pudding`] = [entry(`${home}/Projects/pudding`, 'package.json', false, 1024, 4)];

export const demoInfo: AppInfo = { home, platform: 'demo', places, roots: [{ id: 'root', name: '文件系统', path: '/' }] };

function getFolder(path: string): FileEntry[] {
  if (!folders[path]) throw new Error('演示目录不存在，请选择侧栏中的文件夹。');
  return folders[path];
}

function find(path: string): FileEntry {
  const item = Object.values(folders).flat().find((e) => e.path === path);
  if (!item) throw new Error('此项目已不存在。');
  return item;
}

function cloneTree(source: string, target: string) {
  if (imageContents[source]) imageContents[target] = imageContents[source];
  if (!folders[source]) return;
  folders[target] = folders[source].map((e) => ({ ...e, path: `${target}/${e.name}` }));
  for (const child of folders[source]) cloneTree(child.path, `${target}/${child.name}`);
}

function remove(path: string) {
  for (const image of Object.keys(imageContents)) if (image === path || image.startsWith(`${path}/`)) delete imageContents[image];
  for (const parent of Object.keys(folders)) {
    if (parent === path || parent.startsWith(`${path}/`)) delete folders[parent];
    else {
      const entries = folders[parent];
      const index = entries.findIndex((e) => e.path === path);
      if (index >= 0) entries.splice(index, 1);
    }
  }
}

export async function demoInvoke<T>(command: string, args: Record<string, unknown> = {}): Promise<T> {
  let result: unknown;
  if (command === 'get_app_info') result = demoInfo;
  else if (command === 'read_directory') {
    const path = String(args.path).replace(/^~(?=\/|$)/, home).replace(/\/$/, '') || '/';
    result = { path, parent: path === '/' ? null : path.slice(0, path.lastIndexOf('/')) || '/', entries: structuredClone(getFolder(path)), skipped: 0 } satisfies DirectoryListing;
  } else if (command === 'create_folder' || command === 'rename_entry') {
    const name = String(args.name);
    const invalid = validateName(name);
    if (invalid) throw new Error(invalid);
    const oldPath = String(args.path);
    const parent = command === 'create_folder' ? String(args.parent) : oldPath.slice(0, oldPath.lastIndexOf('/')) || '/';
    const entries = getFolder(parent);
    const path = `${parent === '/' ? '' : parent}/${name}`;
    if (entries.some((e) => e.name === name)) throw new Error('已存在同名项目。');
    if (command === 'create_folder') {
      entries.push(entry(parent === '/' ? '' : parent, name, true));
      folders[path] = [];
    } else {
      const item = find(oldPath);
      cloneTree(oldPath, path);
      entries.push({ ...item, name, path, extension: item.isDir ? '' : name.split('.').slice(1).pop() ?? '' });
      remove(oldPath);
    }
    result = path;
  } else if (command === 'transfer_entries') {
    const paths = args.paths as string[];
    const target = String(args.destination);
    const entries = getFolder(target);
    for (const path of paths) {
      if (target === path || target.startsWith(`${path}/`)) throw new Error('不能将文件夹放入其自身或子文件夹。');
      if (entries.some((e) => e.name === basename(path))) throw new Error('目标中已存在同名项目。');
    }
    result = paths.map((path) => {
      const item = find(path);
      const destination = `${target === '/' ? '' : target}/${item.name}`;
      cloneTree(path, destination);
      entries.push({ ...item, path: destination });
      if (args.mode === 'cut') remove(path);
      return destination;
    });
  } else if (command === 'trash_entries') {
    for (const path of args.paths as string[]) remove(path);
    result = undefined;
  } else if (command === 'read_image') {
    const item = find(String(args.path));
    if (!isImage(item)) throw new Error('此文件格式不支持图片预览。');
    const content = imageContents[item.path];
    if (!content) throw new Error('演示中没有此图片的内容。');
    result = new TextEncoder().encode(content).buffer;
  } else if (command === 'open_entry') {
    throw new Error('浏览器演示无法启动本地应用，请在 Tauri 桌面版中打开文件。');
  } else throw new Error('未知操作。');
  return result as T;
}
