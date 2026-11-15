import { afterEach, expect, it, vi } from 'vitest';
import { api } from '../src/lib/api';
import { loadImageThumbnail } from '../src/lib/image-thumbnail';

afterEach(() => { vi.restoreAllMocks(); vi.unstubAllGlobals(); });

it('resizes images, caches small results, and invalidates the cache when metadata changes', async () => {
  const original = new Blob(['original']);
  const small = new Blob(['small']);
  const read = vi.spyOn(api, 'image').mockResolvedValue('blob:original');
  const revoke = vi.spyOn(URL, 'revokeObjectURL').mockImplementation(() => {});
  const makeUrl = vi.spyOn(URL, 'createObjectURL').mockReturnValue('blob:small');
  const close = vi.fn();
  vi.stubGlobal('fetch', vi.fn().mockResolvedValue({ blob: async () => original }));
  vi.stubGlobal('createImageBitmap', vi.fn().mockResolvedValue({ width: 4000, height: 2000, close }));
  const draw = vi.fn();
  vi.spyOn(HTMLCanvasElement.prototype, 'getContext').mockReturnValue({ drawImage: draw } as unknown as CanvasRenderingContext2D);
  vi.spyOn(HTMLCanvasElement.prototype, 'toBlob').mockImplementation(function (callback) { callback(small); });

  const load = (modified: number) => loadImageThumbnail('/cache-test.png', 'png', modified, 42, () => true);
  expect(await load(1)).toBe('blob:small');
  expect(draw).toHaveBeenCalledWith(expect.anything(), 0, 0, 256, 128);
  expect(revoke).toHaveBeenCalledWith('blob:original');
  expect(close).toHaveBeenCalledOnce();
  expect(await load(1)).toBe('blob:small');
  expect(read).toHaveBeenCalledTimes(1);
  expect(makeUrl).toHaveBeenCalledWith(small);
  await load(2);
  expect(read).toHaveBeenCalledTimes(2);
});

it('releases the original if the item leaves view during decoding', async () => {
  vi.spyOn(api, 'image').mockResolvedValue('blob:late');
  const revoke = vi.spyOn(URL, 'revokeObjectURL').mockImplementation(() => {});
  vi.stubGlobal('fetch', vi.fn().mockResolvedValue({ blob: async () => new Blob() }));
  let finish!: (bitmap: { width: number; height: number; close: () => void }) => void;
  vi.stubGlobal('createImageBitmap', vi.fn(() => new Promise((resolve) => finish = resolve)));
  let active = true;
  const result = loadImageThumbnail('/late-test.png', 'png', 1, 1, () => active);
  await vi.waitFor(() => expect(finish).toBeDefined());
  active = false;
  const close = vi.fn();
  finish({ width: 100, height: 100, close });
  expect(await result).toBeNull();
  expect(close).toHaveBeenCalledOnce();
  expect(revoke).toHaveBeenCalledWith('blob:late');
});
