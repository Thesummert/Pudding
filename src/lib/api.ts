import { invoke, isTauri } from '@tauri-apps/api/core';
import type { AppInfo, Clipboard, DirectoryListing } from './types';
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
  startDrag: (paths: string[]) => invoke<void>('start_file_drag', { paths }),
  listenDrop: (handler: (event: DragDropEvent) => void): Promise<() => void> => desktop
    ? getCurrentWebview().onDragDropEvent((event) => handler(event.payload))
    : Promise.resolve(() => {}),
  info: () => call<AppInfo>('get_app_info'),
  read: (path: string) => call<DirectoryListing>('read_directory', { path }),
  create: (parent: string, name: string) => call<string>('create_folder', { parent, name }),
  rename: (path: string, name: string) => call<string>('rename_entry', { path, name }),
  transfer: (paths: string[], destination: string, mode: 'copy' | 'cut') => call<string[]>('transfer_entries', { paths, destination, mode }),
  trash: (paths: string[]) => call<void>('trash_entries', { paths }),
  open: (path: string) => call<void>('open_entry', { path }),
  image: async (path: string, extension: string): Promise<string> => {
    const bytes = await call<ArrayBuffer | number[]>('read_image', { path });
    return URL.createObjectURL(new Blob([bytes instanceof ArrayBuffer ? bytes : new Uint8Array(bytes)], { type: imageMimeType(extension) }));
  },
};
