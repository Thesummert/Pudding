import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { flushSync, mount, tick, unmount } from 'svelte';
import App from '../src/App.svelte';
import { api } from '../src/lib/api';

vi.mock('../src/lib/api', async (original) => {
  const module = await original<typeof import('../src/lib/api')>();
  return { ...module, desktop: true, api: { ...module.api, info: async () => ({ ...await module.api.info(), platform: 'macos' }), connectSmb: vi.fn(), listSmbShares: vi.fn() } };
});

let host: HTMLDivElement;
let app: ReturnType<typeof mount>;

beforeEach(async () => {
  localStorage.clear();
  vi.mocked(api.connectSmb).mockReset();
  vi.mocked(api.listSmbShares).mockReset();
  host = document.createElement('div');
  document.body.append(host);
  app = mount(App, { target: host });
  flushSync();
  await vi.waitFor(() => expect(host.querySelectorAll('.file-card')).toHaveLength(10));
});
afterEach(async () => { await unmount(app); host.remove(); });

it('saves successful connections without credentials and navigates to the share', async () => {
  const read = vi.spyOn(api, 'read');
  vi.mocked(api.connectSmb).mockResolvedValue(['/home/pudding/Documents']);
  [...host.querySelectorAll('button')].find((button) => button.textContent?.includes('连接 SMB 共享'))!.click();
  const inputs = host.querySelectorAll<HTMLInputElement>('.smb-address input');
  for (const [index, value] of ['smb://server/share', 'alice', 'secret'].entries()) {
    inputs[index].value = value;
    inputs[index].dispatchEvent(new Event('input', { bubbles: true }));
  }
  await tick();
  host.querySelector<HTMLFormElement>('.smb-modal form')!.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }));
  await vi.waitFor(() => expect(api.connectSmb).toHaveBeenCalled());
  await tick();
  expect(host.querySelector('.smb-error')?.textContent).toBeUndefined();
  await vi.waitFor(() => expect(read).toHaveBeenCalledWith('/home/pudding/Documents'));
  await vi.waitFor(() => expect(host.querySelector('h1')?.textContent).toBe('文档.'));
  expect(api.connectSmb).toHaveBeenCalledWith('smb://server/share', 'alice', 'secret');
  expect(localStorage.getItem('pudding.smbHistory')).toBe('["smb://server/share"]');
  await tick();
  expect(inputs[2].value).toBe('');
  [...host.querySelectorAll('button')].find((button) => button.textContent?.includes('连接 SMB 共享'))!.click();
  await tick();
  host.querySelector<HTMLButtonElement>('[aria-label="删除历史 smb://server/share"]')!.click();
  await tick();
  expect(localStorage.getItem('pudding.smbHistory')).toBe('[]');
});

it('does not save failed connections', async () => {
  vi.mocked(api.listSmbShares).mockRejectedValue(new Error('无法连接'));
  [...host.querySelectorAll('button')].find((button) => button.textContent?.includes('连接 SMB 共享'))!.click();
  const address = host.querySelector<HTMLInputElement>('.smb-address input')!;
  address.value = 'smb://server';
  address.dispatchEvent(new Event('input', { bubbles: true }));
  host.querySelector<HTMLFormElement>('.smb-modal form')!.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }));
  await vi.waitFor(() => expect(host.querySelector('.smb-error')?.textContent).toContain('无法连接'));
  expect(localStorage.getItem('pudding.smbHistory')).toBeNull();
});

it('lists shares inside the dialog and connects to the chosen share', async () => {
  vi.mocked(api.listSmbShares).mockResolvedValue(['共享 文件']);
  vi.mocked(api.connectSmb).mockResolvedValue(['/home/pudding/Documents']);
  [...host.querySelectorAll('button')].find((button) => button.textContent?.includes('连接 SMB 共享'))!.click();
  const address = host.querySelector<HTMLInputElement>('.smb-address input')!;
  address.value = 'smb://server';
  address.dispatchEvent(new Event('input', { bubbles: true }));
  await tick();
  const form = host.querySelector<HTMLFormElement>('.smb-modal form')!;
  form.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }));
  await vi.waitFor(() => expect(host.textContent).toContain('共享 文件'));
  expect(api.listSmbShares).toHaveBeenCalledWith('smb://server', '', '');
  [...host.querySelectorAll<HTMLButtonElement>('.smb-history-address')].find((button) => button.textContent?.includes('共享 文件'))!.click();
  await tick();
  form.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }));
  await vi.waitFor(() => expect(api.connectSmb).toHaveBeenCalledWith('smb://server/%E5%85%B1%E4%BA%AB%20%E6%96%87%E4%BB%B6', '', ''));
});
