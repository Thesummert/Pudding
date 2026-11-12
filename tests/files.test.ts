import { describe, expect, it } from 'vitest';
import { breadcrumbs, formatSize, validateName, visibleEntries } from '../src/lib/files';
import type { FileEntry } from '../src/lib/types';

const entry = (name: string, properties: Partial<FileEntry> = {}): FileEntry => ({ name, path: `/test/${name}`, isDir: false, isSymlink: false, size: 0, modified: null, hidden: false, extension: '', ...properties });

describe('file presentation', () => {
  it('keeps directories first in either sort direction and uses natural numeric ordering', () => {
    const entries = [entry('file10'), entry('folder', { isDir: true }), entry('file2')];
    expect(visibleEntries(entries, '', false, 'name', false).map((e) => e.name)).toEqual(['folder', 'file2', 'file10']);
    expect(visibleEntries(entries, '', false, 'name', true).map((e) => e.name)).toEqual(['folder', 'file10', 'file2']);
    expect(entries[0].name).toBe('file10');
  });

  it('combines case-insensitive local filtering with hidden-file visibility', () => {
    const entries = [entry('README.md'), entry('.readme', { hidden: true }), entry('other')];
    expect(visibleEntries(entries, ' readme ', false, 'name', false)).toHaveLength(1);
    expect(visibleEntries(entries, 'readme', true, 'name', false)).toHaveLength(2);
  });

  it('formats byte sizes without mislabelling empty files', () => {
    expect(formatSize(0)).toBe('0 B');
    expect(formatSize(1024)).toBe('1 KB');
    expect(formatSize(1536)).toBe('1.5 KB');
    expect(formatSize(1024 ** 3)).toBe('1 GB');
  });

  it('builds navigable POSIX, Windows drive and UNC breadcrumbs', () => {
    expect(breadcrumbs('/home/pudding')[2]).toEqual({ name: 'pudding', path: '/home/pudding' });
    expect(breadcrumbs('/')).toEqual([{ name: '/', path: '/' }]);
    expect(breadcrumbs('C:\\Users\\Alice')).toEqual([{ name: 'C:', path: 'C:/' }, { name: 'Users', path: 'C:/Users' }, { name: 'Alice', path: 'C:/Users/Alice' }]);
    expect(breadcrumbs('\\\\server\\share\\folder')[2].path).toBe('//server/share/folder');
  });

  it('rejects path traversal and Windows reserved filenames', () => {
    for (const name of ['', ' ', '.', '..', '../folder', 'a/b', 'a\\b', 'CON.txt', 'lpt9', 'name.', 'name ', 'bad\0name']) expect(validateName(name), name).not.toBeNull();
    expect(validateName('资料 2026')).toBeNull();
    expect(validateName('report.final.md')).toBeNull();
  });
});
