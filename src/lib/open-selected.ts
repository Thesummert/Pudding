import type { FileEntry } from './types';

export async function openSelectedFiles(entries: FileEntry[], open: (path: string) => Promise<void>) {
  const failures: { name: string; error: unknown }[] = [];
  const files = entries.filter((entry) => !entry.isDir);
  for (const file of files) {
    try { await open(file.path); }
    catch (error) { failures.push({ name: file.name, error }); }
  }
  return { attempted: files.length, skipped: entries.length - files.length, failures };
}
