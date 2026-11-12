import { invoke, isTauri } from '@tauri-apps/api/core';
import type { AppInfo, DirectoryListing } from './types';
import { imageMimeType } from './files';

export const desktop = isTauri();

async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (desktop) return invoke<T>(command, args);
  const { demoInvoke } = await import('./demo');
  return demoInvoke<T>(command, args);
}

export const api = {
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
