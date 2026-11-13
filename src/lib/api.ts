import { listen } from '@tauri-apps/api/event';
import { convertFileSrc, invoke, isTauri } from '@tauri-apps/api/core';
import type { AppInfo, Clipboard, DirectoryListing, FileEntry, Volume } from './types';
import { getCurrentWebview, type DragDropEvent } from '@tauri-apps/api/webview';
import { imageMimeType } from './files';

export const desktop = isTauri();
let demoClipboard: Clipboard = null;

async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (desktop) return invoke<T>(command, args);
  const { demoInvoke } = await import('./demo');
  return demoInvoke<T>(command, args);
}

export const api = {
  readClipboard: () => desktop ? invoke<Clipboard>('read_file_clipboard') : Promise.resolve(demoClipboard),
  writeClipboard: async (operation: NonNullable<Clipboard>) => {
    if (desktop) await invoke('write_file_clipboard', { operation });
    else demoClipboard = structuredClone(operation);
  },
  clearClipboard: async (expected: NonNullable<Clipboard>) => {
    if (desktop) await invoke('clear_file_clipboard', { expected });
    else if (JSON.stringify(demoClipboard) === JSON.stringify(expected)) demoClipboard = null;
  },
  writeText: async (text: string) => {
    if (desktop) await invoke('write_clipboard_text', { text });
    else { await navigator.clipboard.writeText(text); demoClipboard = null; }
  },
  dropModifiers: () => desktop ? invoke<{ ctrl: boolean; shift: boolean }>('drop_modifiers') : Promise.resolve({ ctrl: false, shift: false }),
  startDrag: (paths: string[]) => invoke<void>('start_file_drag', { paths }),
  listenDrop: (handler: (event: DragDropEvent) => void): Promise<() => void> => desktop
    ? getCurrentWebview().onDragDropEvent((event) => handler(event.payload))
    : Promise.resolve(() => {}),
  openWindow: async (path: string) => {
    if (desktop) await invoke('open_directory_window', { path });
    else { const url = new URL(window.location.href); url.searchParams.set('path', path); window.open(url.toString(), '_blank'); }
  },
  watch: (path: string) => desktop ? invoke<void>('watch_directory', { path }) : Promise.resolve(),
  listenChanges: (handler: () => void) => desktop ? listen('filesystem-changed', handler) : Promise.resolve(() => {}),
  listenSearchProgress: (handler: (progress: [number, number]) => void) => desktop ? listen<[number, number]>('search-progress', (event) => handler(event.payload)) : Promise.resolve(() => {}),
  search: (path: string, query: string, hidden: boolean, searchContent: boolean, requestId: number) => desktop ? invoke<FileEntry[]>('search_files', { path, query, hidden, searchContent, requestId }) : Promise.resolve<FileEntry[] | null>(null),
  info: () => call<AppInfo>('get_app_info'),
  volumes: () => call<Volume[]>('list_volumes'),
  ejectVolume: (path: string) => call<void>('eject_volume', { path }),
  connectSmb: (address: string, username: string, password: string) => desktop ? invoke<string[]>('connect_smb', { address, username, password }) : Promise.reject(new Error('浏览器演示无法连接 SMB 共享。')),
  listSmbShares: (address: string, username: string, password: string) => desktop ? invoke<string[]>('list_smb_shares', { address, username, password }) : Promise.reject(new Error('浏览器演示无法连接 SMB 共享。')),
  startupDirectory: () => desktop ? invoke<string | null>('startup_directory') : Promise.resolve(null),
  takeOpenDirectories: () => desktop ? invoke<string[]>('take_open_directories') : Promise.resolve([]),
  listenOpenDirectories: (handler: () => void) => desktop ? getCurrentWebview().listen('open-directories', handler) : Promise.resolve(() => {}),
  setWindowDirectory: (path: string) => desktop ? invoke<void>('set_window_directory', { path }) : Promise.resolve(),
  read: (path: string) => call<DirectoryListing>('read_directory', { path }),
  readEntryDetails: (directory: string, names: string[]) => call<FileEntry[]>('read_entry_details', { directory, names }),
  directorySize: async (path: string): Promise<number> => {
    if (desktop) return invoke<number>('directory_size', { path });
    const listing = await call<DirectoryListing>('read_directory', { path });
    let total = 0;
    for (const entry of listing.entries) {
      if (!entry.isSymlink) total += entry.isDir ? await api.directorySize(entry.path) : entry.size;
    }
    return total;
  },
  create: (parent: string, name: string) => call<string>('create_folder', { parent, name }),
  createFile: (parent: string, name: string) => call<string>('create_file', { parent, name }),
  rename: (path: string, name: string) => call<string>('rename_entry', { path, name }),
  transferConflicts: (paths: string[], destination: string) => call<string[]>('transfer_conflicts', { paths, destination }),
  transfer: (paths: string[], destination: string, mode: 'copy' | 'cut', policy: 'reject' | 'replace' | 'keep' = 'reject', confirmed: string[] = []) => call<string[]>('transfer_entries', { paths, destination, mode, policy, confirmed }),
  trash: (paths: string[]) => call<void>('trash_entries', { paths }),
  delete: (paths: string[]) => call<void>('delete_entries', { paths }),
  open: (path: string) => call<void>('open_entry', { path }),
  openSystemFileManager: (path: string) => desktop ? invoke<void>('open_system_file_manager', { path }) : Promise.reject(new Error('浏览器演示无法打开系统文件管理器。')),
  listOpenApps: (path: string) => desktop ? invoke<{ id: string; name: string }[]>('list_open_apps', { path }) : Promise.resolve([]),
  openWith: (path: string, application: string | null = null) => desktop ? invoke<void>('open_entry_with', { path, application }) : Promise.reject(new Error('浏览器演示无法启动本地应用。')),
  sugarAvailable: () => desktop ? invoke<boolean>('sugar_available') : Promise.resolve(false),
  sugarArchive: (action: string, paths: string[]) => invoke<void>('sugar_archive', { action, paths }),
  openTerminal: (path: string, terminal: string, custom: string) => desktop ? invoke<void>('open_terminal', { path, terminal, custom }) : Promise.reject(new Error('浏览器演示无法打开终端。')),
  chooseTerminal: () => desktop ? invoke<string | null>('choose_terminal') : Promise.reject(new Error('浏览器演示无法选择终端应用。')),
  previewBytes: async (path: string, kind: 'text' | 'pdf'): Promise<Uint8Array> => {
    const bytes = await call<ArrayBuffer | number[]>('read_preview_file', { path, kind });
    return new Uint8Array(bytes);
  },
  videoSource: async (path: string): Promise<string> => {
    if (!desktop) return call<string>('video_source', { path });
    return convertFileSrc(await invoke<string>('prepare_video_preview', { path }));
  },
  image: async (path: string, extension: string): Promise<string> => {
    const bytes = await call<ArrayBuffer | number[]>('read_image', { path });
    return URL.createObjectURL(new Blob([bytes instanceof ArrayBuffer ? bytes : new Uint8Array(bytes)], { type: imageMimeType(extension) }));
  },
};
