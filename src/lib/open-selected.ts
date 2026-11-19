import type { FileEntry } from './types';

export async function openSelectedFiles(entries: FileEntry[], open: (path: string) => Promise<void>) {
  const failures: { name: string; error: unknown }[] = [];
  const items = entries.filter((entry) => !entry.isDir || entry.isApp);
  for (const item of items) {
    try { await open(item.path); }
    catch (error) { failures.push({ name: item.name, error }); }
  }
  return { attempted: items.length, skipped: entries.length - items.length, failures };
}
