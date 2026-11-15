import { describe, expect, it, vi } from 'vitest';
import { openSelectedFiles } from '../src/lib/open-selected';
import type { FileEntry } from '../src/lib/types';

function entry(name: string, isDir = false): FileEntry {
  return { name, path: `/${name}`, isDir, isSymlink: false, size: 0, modified: null, hidden: false, extension: '' };
}

describe('openSelectedFiles', () => {
  it('opens every file even when one fails and skips folders', async () => {
    const open = vi.fn(async (path: string) => { if (path === '/one') throw new Error('失败'); });
    const result = await openSelectedFiles([entry('one'), entry('folder', true), entry('two')], open);
    expect(open.mock.calls).toEqual([['/one'], ['/two']]);
    expect(result.attempted).toBe(2);
    expect(result.skipped).toBe(1);
    expect(result.failures).toEqual([{ name: 'one', error: expect.any(Error) }]);
  });
});
