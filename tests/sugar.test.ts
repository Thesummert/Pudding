import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { flushSync, mount, tick, unmount } from 'svelte';
import App from '../src/App.svelte';
import { api } from '../src/lib/api';

vi.mock('../src/lib/api', async (original) => {
  const module = await original<typeof import('../src/lib/api')>();
  return { ...module, desktop: true, api: { ...module.api, sugarAvailable: vi.fn(), sugarArchive: vi.fn() } };
});
let host: HTMLDivElement;
let app: ReturnType<typeof mount>;
beforeEach(async () => {
  localStorage.clear();
  vi.mocked(api.sugarAvailable).mockReset();
  vi.mocked(api.sugarArchive).mockReset();
  host = document.createElement('div'); document.body.append(host);
  app = mount(App, { target: host }); flushSync();
  await vi.waitFor(() => expect(host.querySelectorAll('.file-card')).toHaveLength(10));
});
afterEach(async () => { await unmount(app); host.remove(); });
it('offers compression only when the Sugar helper is available', async () => {
  vi.mocked(api.sugarAvailable).mockResolvedValue(true);
  vi.mocked(api.sugarArchive).mockResolvedValue();
  host.querySelector<HTMLButtonElement>('.file-card')!.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true }));
  await vi.waitFor(() => expect(host.textContent).toContain('解压缩'));
  [...host.querySelectorAll<HTMLButtonElement>('[role="menuitem"]')].find((item) => item.textContent?.includes('解压缩'))!.click();
  await tick();
  expect(host.textContent).toContain('快速压缩为 ZIP');
  [...host.querySelectorAll<HTMLButtonElement>('[role="menuitem"]')].find((item) => item.textContent?.includes('快速压缩为 ZIP'))!.click();
  await vi.waitFor(() => expect(api.sugarArchive).toHaveBeenCalledWith('compress', expect.arrayContaining([expect.any(String)])));
});
it('hides archive operations when Sugar is not installed', async () => {
  vi.mocked(api.sugarAvailable).mockResolvedValue(false);
  host.querySelector<HTMLButtonElement>('.file-card')!.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true }));
  await tick(); await tick();
  expect(host.textContent).not.toContain('解压缩');
});

it('opens custom compression for the current folder from empty space', async () => {
  vi.mocked(api.sugarAvailable).mockResolvedValue(true);
  vi.mocked(api.sugarArchive).mockResolvedValue();
  host.querySelector<HTMLElement>('.file-area')!.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true }));
  await vi.waitFor(() => expect(host.textContent).toContain('解压缩'));
  [...host.querySelectorAll<HTMLButtonElement>('[role="menuitem"]')].find((item) => item.textContent?.includes('解压缩'))!.click();
  await tick();
  expect(host.textContent).toContain('快速压缩当前文件夹');
  [...host.querySelectorAll<HTMLButtonElement>('[role="menuitem"]')].find((item) => item.textContent?.includes('自定义压缩'))!.click();
  await vi.waitFor(() => expect(api.sugarArchive).toHaveBeenCalledWith('custom', [expect.any(String)]));
});
