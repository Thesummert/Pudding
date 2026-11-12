import { afterEach, expect, it, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import OpenWithDialog from '../src/lib/OpenWithDialog.svelte';
import { api } from '../src/lib/api';

const entry = { name: 'image.png', path: '/tmp/image.png', isDir: false, isSymlink: false, size: 1, modified: null, hidden: false, extension: 'png' };
let app: ReturnType<typeof mount>;
let host: HTMLDivElement;
afterEach(async () => { if (app) await unmount(app); host?.remove(); vi.restoreAllMocks(); });

it('shows recommended apps and opens the selected file with one', async () => {
  vi.spyOn(api, 'listOpenApps').mockResolvedValue([{ id: 'viewer.desktop', name: '图片查看器' }]);
  const open = vi.spyOn(api, 'openWith').mockResolvedValue();
  const close = vi.fn();
  host = document.createElement('div'); document.body.append(host);
  app = mount(OpenWithDialog, { target: host, props: { entry, onclose: close } }); flushSync();
  await vi.waitFor(() => expect(host.textContent).toContain('图片查看器'));
  host.querySelector<HTMLButtonElement>('.open-with-app')!.click();
  await vi.waitFor(() => expect(open).toHaveBeenCalledWith(entry.path, 'viewer.desktop'));
  await vi.waitFor(() => expect(close).toHaveBeenCalled());
});

it('keeps manual selection available if recommendations fail', async () => {
  vi.spyOn(api, 'listOpenApps').mockRejectedValue(new Error('查询失败'));
  const open = vi.spyOn(api, 'openWith').mockResolvedValue();
  host = document.createElement('div'); document.body.append(host);
  app = mount(OpenWithDialog, { target: host, props: { entry, onclose: vi.fn() } }); flushSync();
  await vi.waitFor(() => expect(host.textContent).toContain('查询失败'));
  [...host.querySelectorAll('button')].find((button) => button.textContent?.includes('选择其他应用'))!.click();
  await vi.waitFor(() => expect(open).toHaveBeenCalledWith(entry.path, null));
});
