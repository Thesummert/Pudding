import type { AppInfo, DirectoryListing, FileEntry } from './types';
import { basename, isImage, validateName } from './files';
import coast from './demo-images/coast.svg?raw';
import mountains from './demo-images/mountains.svg?raw';
import sampleVideo from './demo-videos/sample.mp4?url';

const home = '/home/pudding';
const now = Math.floor(Date.now() / 1000);
const folders: Record<string, FileEntry[]> = {};
const textContents: Record<string, string> = {};
const videoContents: Record<string, string> = { [`${home}/Videos/示例视频.mp4`]: sampleVideo };
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
folders[`${home}/Videos`] = [entry(`${home}/Videos`, '示例视频.mp4', false, 2048)];
folders[`${home}/Projects`] = [entry(`${home}/Projects`, 'pudding', true), entry(`${home}/Projects`, 'README.md', false, 3120, 2)];
folders[`${home}/Projects/pudding`] = [entry(`${home}/Projects/pudding`, 'package.json', false, 1024, 4)];

export const demoInfo: AppInfo = { home, platform: 'demo', places, roots: [{ id: 'root', name: '文件系统', path: '/', ejectable: false }] };

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
  if (Object.hasOwn(textContents, source)) textContents[target] = textContents[source];
  if (imageContents[source]) imageContents[target] = imageContents[source];
  if (videoContents[source]) videoContents[target] = videoContents[source];
  if (!folders[source]) return;
  folders[target] = folders[source].map((e) => ({ ...e, path: `${target}/${e.name}` }));
  for (const child of folders[source]) cloneTree(child.path, `${target}/${child.name}`);
}

function remove(path: string) {
  for (const text of Object.keys(textContents)) if (text === path || text.startsWith(`${path}/`)) delete textContents[text];
  for (const image of Object.keys(imageContents)) if (image === path || image.startsWith(`${path}/`)) delete imageContents[image];
  for (const video of Object.keys(videoContents)) if (video === path || video.startsWith(`${path}/`)) delete videoContents[video];
  for (const parent of Object.keys(folders)) {
    if (parent === path || parent.startsWith(`${path}/`)) delete folders[parent];
    else {
      const entries = folders[parent];
      const index = entries.findIndex((e) => e.path === path);
      if (index >= 0) entries.splice(index, 1);
    }
  }
}

function demoPdf(): ArrayBuffer {
  const content = 'BT /F1 22 Tf 72 720 Td (Pudding PDF Preview) Tj ET';
  const second = 'BT /F2 22 Tf 72 720 Td <4F60597D> Tj ET';
  const objects = [
    '<< /Type /Catalog /Pages 2 0 R >>',
    '<< /Type /Pages /Kids [3 0 R 6 0 R] /Count 2 >>',
    '<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>',
    '<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>',
    `<< /Length ${content.length} >>\nstream\n${content}\nendstream`,
    '<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F2 8 0 R >> >> /Contents 7 0 R >>',
    `<< /Length ${second.length} >>\nstream\n${second}\nendstream`,
    '<< /Type /Font /Subtype /Type0 /BaseFont /STSong-Light /Encoding /UniGB-UCS2-H /DescendantFonts [9 0 R] >>',
    '<< /Type /Font /Subtype /CIDFontType0 /BaseFont /STSong-Light /CIDSystemInfo << /Registry (Adobe) /Ordering (GB1) /Supplement 2 >> /FontDescriptor 10 0 R >>',
    '<< /Type /FontDescriptor /FontName /STSong-Light /Flags 6 /FontBBox [0 -200 1000 900] /ItalicAngle 0 /Ascent 800 /Descent -200 /CapHeight 700 /StemV 80 >>',
  ];
  let pdf = '%PDF-1.4\n';
  const offsets = objects.map((object, index) => {
    const offset = pdf.length;
    pdf += `${index + 1} 0 obj\n${object}\nendobj\n`;
    return offset;
  });
  const xref = pdf.length;
  pdf += `xref\n0 ${objects.length + 1}\n0000000000 65535 f \n`;
  for (const offset of offsets) pdf += `${String(offset).padStart(10, '0')} 00000 n \n`;
  pdf += `trailer\n<< /Size ${objects.length + 1} /Root 1 0 R >>\nstartxref\n${xref}\n%%EOF\n`;
  return new TextEncoder().encode(pdf).buffer;
}

export async function demoInvoke<T>(command: string, args: Record<string, unknown> = {}): Promise<T> {
  let result: unknown;
  if (command === 'get_app_info') result = demoInfo;
  else if (command === 'list_volumes') result = structuredClone(demoInfo.roots);
  else if (command === 'read_directory') {
    const path = String(args.path).replace(/^~(?=\/|$)/, home).replace(/\/$/, '') || '/';
    result = { path, parent: path === '/' ? null : path.slice(0, path.lastIndexOf('/')) || '/', entries: structuredClone(getFolder(path)), skipped: 0 } satisfies DirectoryListing;
  } else if (command === 'create_folder' || command === 'create_file' || command === 'rename_entry') {
    const name = String(args.name);
    const invalid = validateName(name);
    if (invalid) throw new Error(invalid);
    const oldPath = String(args.path);
    const parent = command !== 'rename_entry' ? String(args.parent) : oldPath.slice(0, oldPath.lastIndexOf('/')) || '/';
    const entries = getFolder(parent);
    const path = `${parent === '/' ? '' : parent}/${name}`;
    if (entries.some((e) => e.name === name)) throw new Error('已存在同名项目。');
    if (command === 'create_folder') {
      entries.push(entry(parent === '/' ? '' : parent, name, true));
      folders[path] = [];
    } else if (command === 'create_file') {
      entries.push(entry(parent === '/' ? '' : parent, name, false));
      textContents[path] = '';
    } else {
      const item = find(oldPath);
      cloneTree(oldPath, path);
      entries.push({ ...item, name, path, extension: item.isDir ? '' : name.split('.').slice(1).pop() ?? '' });
      remove(oldPath);
    }
    result = path;
  } else if (command === 'transfer_conflicts' || command === 'transfer_entries') {
    const paths = args.paths as string[];
    const target = String(args.destination);
    const entries = getFolder(target);
    const targets: string[] = [];
    const conflicts: string[] = [];
    for (const path of paths) {
      if (target === path || target.startsWith(`${path}/`)) throw new Error('不能将文件夹放入其自身或子文件夹。');
      find(path);
      const destination = `${target === '/' ? '' : target}/${basename(path)}`;
      if (entries.some((e) => e.path === destination) || targets.includes(destination)) conflicts.push(destination);
      targets.push(destination);
    }
    if (command === 'transfer_conflicts') result = conflicts;
    else {
      const policy = String(args.policy ?? 'reject');
      const confirmed = (args.confirmed ?? []) as string[];
      const reserved: string[] = [];
      const plan = paths.map((path, index) => {
        const item = find(path);
        let destination = targets[index];
        if (entries.some((e) => e.path === destination) || reserved.includes(destination)) {
          if (policy === 'keep') {
            const dot = !item.isDir && !item.name.startsWith('.') ? item.name.lastIndexOf('.') : -1;
            const stem = dot > 0 ? item.name.slice(0, dot) : item.name;
            const extension = dot > 0 ? item.name.slice(dot) : '';
            let n = 1;
            do { destination = `${target === '/' ? '' : target}/${stem} 副本${n === 1 ? '' : ` ${n}`}${extension}`; n++; }
            while (entries.some((e) => e.path === destination) || reserved.includes(destination));
          } else if (policy !== 'replace' || paths.includes(destination) || reserved.includes(destination) || !confirmed.includes(destination)) throw new Error('目标中已存在同名项目。');
        }
        reserved.push(destination);
        return { path, item: { ...item }, destination };
      });
      result = plan.map(({ path, item, destination }) => {
        if (entries.some((e) => e.path === destination)) remove(destination);
        cloneTree(path, destination);
        entries.push({ ...item, name: basename(destination), path: destination });
        if (args.mode === 'cut') remove(path);
        return destination;
      });
    }
  } else if (command === 'trash_entries' || command === 'delete_entries') {
    for (const path of args.paths as string[]) remove(path);
    result = undefined;
  } else if (command === 'video_source') {
    const item = find(String(args.path));
    const source = videoContents[item.path];
    if (!source) throw new Error('演示中没有此视频的内容。');
    result = source;
  } else if (command === 'read_preview_file') {
    const item = find(String(args.path));
    if (args.kind === 'pdf' && item.extension.toLowerCase() === 'pdf') result = demoPdf();
    else if (args.kind === 'text') result = new TextEncoder().encode(textContents[item.path] ?? `这是「${item.name}」的演示内容。\n`).buffer;
    else throw new Error('演示中没有此文件的预览内容。');
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
