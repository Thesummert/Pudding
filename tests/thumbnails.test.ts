import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { mount, tick, unmount } from 'svelte';
import FileThumbnail from '../src/lib/FileThumbnail.svelte';
import { api } from '../src/lib/api';
import type { FileEntry } from '../src/lib/types';

const entry: FileEntry = { name: '图片.png', path: '/pictures/图片.png', extension: 'png', isDir: false, isSymlink: false, hidden: false, modified: 1, size: 100 };
let app: ReturnType<typeof mount>;
let host: HTMLDivElement;
let intersect: (visible: boolean) => void;
let disconnect: ReturnType<typeof vi.fn>;

beforeEach(async () => {
  disconnect = vi.fn();
  vi.stubGlobal('IntersectionObserver', class {
    constructor(callback: IntersectionObserverCallback) {
      intersect = (visible) => callback([{ isIntersecting: visible } as IntersectionObserverEntry], this as unknown as IntersectionObserver);
    }
    observe() {}
    disconnect = disconnect;
  });
  host = document.createElement('div');
  document.body.append(host);
  app = mount(FileThumbnail, { target: host, props: { entry, large: true } });
  await tick();
});

afterEach(async () => {
  await unmount(app);
  host.remove();
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

describe('image thumbnails', () => {
  it('loads only near the viewport and releases the image when scrolled away', async () => {
    const url = URL.createObjectURL(new Blob(['image']));
    const read = vi.spyOn(api, 'image').mockResolvedValue(url);
    const revoke = vi.spyOn(URL, 'revokeObjectURL');
    expect(read).not.toHaveBeenCalled();
    expect(host.querySelector('.file-icon')).not.toBeNull();
    intersect(true);
    await vi.waitFor(() => expect(host.querySelector('img')?.getAttribute('src')).toBe(url));
    host.querySelector('img')!.dispatchEvent(new Event('load'));
    await tick();
    expect(host.querySelector('.file-icon')).toBeNull();
    intersect(false);
    await tick();
    expect(host.querySelector('img')).toBeNull();
    expect(host.querySelector('.file-icon')).not.toBeNull();
    expect(revoke).toHaveBeenCalledWith(url);
  });

  it('keeps the file icon when reading or decoding the thumbnail fails', async () => {
    const read = vi.spyOn(api, 'image').mockRejectedValueOnce(new Error('图片超过 32 MB'));
    intersect(true);
    await vi.waitFor(() => expect(host.querySelector('.thumbnail')?.getAttribute('title')).toContain('32 MB'));
    expect(host.querySelector('.file-icon')).not.toBeNull();
    expect(host.querySelector('img')).toBeNull();
    intersect(false);
    await tick();
    const url = URL.createObjectURL(new Blob(['broken']));
    read.mockResolvedValueOnce(url);
    intersect(true);
    await vi.waitFor(() => expect(host.querySelector('img')).not.toBeNull());
    host.querySelector('img')!.dispatchEvent(new Event('error'));
    await tick();
    expect(host.querySelector('img')).toBeNull();
    expect(host.querySelector('.file-icon')).not.toBeNull();
    expect(host.querySelector('.thumbnail')?.getAttribute('title')).toContain('图片可能已损坏');
  });

  it('discards a pending read after navigating away and disconnects the observer', async () => {
    const url = URL.createObjectURL(new Blob(['late image']));
    let resolveImage!: (value: string) => void;
    const read = vi.spyOn(api, 'image').mockReturnValue(new Promise<string>((resolve) => resolveImage = resolve));
    const revoke = vi.spyOn(URL, 'revokeObjectURL');
    intersect(true);
    await tick();
    expect(read).toHaveBeenCalledOnce();
    await unmount(app);
    // Keep teardown balanced after explicitly unmounting to simulate navigation.
    app = mount(FileThumbnail, { target: host, props: { entry } });
    await tick();
    expect(disconnect).toHaveBeenCalledOnce();
    resolveImage(url);
    await vi.waitFor(() => expect(revoke).toHaveBeenCalledWith(url));
    expect(host.querySelector('img')).toBeNull();
  });
});


describe('video thumbnails', () => {
  const video: FileEntry = { ...entry, name: '视频.mp4', path: '/videos/视频.mp4', extension: 'mp4' };

  it('loads the first frame only when visible and falls back on decode errors', async () => {
    await unmount(app);
    const load = vi.spyOn(api, 'videoSource').mockResolvedValue('asset://localhost/video');
    app = mount(FileThumbnail, { target: host, props: { entry: video, large: true } });
    await tick();
    expect(load).not.toHaveBeenCalled();
    intersect(true);
    await vi.waitFor(() => expect(host.querySelector('video')).not.toBeNull());
    expect(load).toHaveBeenCalledWith(video.path);
    const player = host.querySelector('video')!;
    expect(player.preload).toBe('auto');
    Object.defineProperty(player, 'duration', { value: 10, configurable: true });
    player.dispatchEvent(new Event('loadedmetadata'));
    expect(player.currentTime).toBe(1);
    expect(host.querySelector('.file-icon')).not.toBeNull();
    player.dispatchEvent(new Event('loadeddata'));
    await tick();
    expect(host.querySelector('.file-icon')).toBeNull();
    host.querySelector('video')!.dispatchEvent(new Event('error'));
    await tick();
    expect(host.querySelector('video')).toBeNull();
    expect(host.querySelector('.file-icon')).not.toBeNull();
    intersect(false);
    await tick();
    expect(host.querySelector('video')).toBeNull();
  });

  it('ignores late video results after the thumbnail leaves view', async () => {
    await unmount(app);
    let resolve!: (url: string) => void;
    vi.spyOn(api, 'videoSource').mockReturnValue(new Promise<string>((done) => resolve = done));
    app = mount(FileThumbnail, { target: host, props: { entry: video } });
    await tick();
    intersect(true);
    await tick();
    intersect(false);
    await tick();
    resolve('asset://localhost/video');
    await tick();
    expect(host.querySelector('video')).toBeNull();
  });
});
