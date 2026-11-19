import { describe, expect, it, vi } from 'vitest';
import { openSelectedFiles } from '../src/lib/open-selected';
import type { FileEntry } from '../src/lib/types';

function entry(name: string, isDir = false): FileEntry {
  return { name, path: `/${name}`, isDir, isSymlink: false, size: 0, modified: null, hidden: false, extension: '' };
}

describe('openSelectedFiles', () => {
  it('opens files and applications even when one fails and skips ordinary folders', async () => {
    const open = vi.fn(async (path: string) => { if (path === '/one') throw new Error('失败'); });
    const app = { ...entry('Example.app', true), isApp: true };
    const result = await openSelectedFiles([entry('one'), entry('folder', true), app, entry('two')], open);
    expect(open.mock.calls).toEqual([['/one'], ['/Example.app'], ['/two']]);
    expect(result.attempted).toBe(3);
    expect(result.skipped).toBe(1);
    expect(result.failures).toEqual([{ name: 'one', error: expect.any(Error) }]);
  });
});
