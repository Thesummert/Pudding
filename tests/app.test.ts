import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { flushSync, mount, tick, unmount } from 'svelte';
import App from '../src/App.svelte';
import { api } from '../src/lib/api';

let app: ReturnType<typeof mount>;
let host: HTMLDivElement;

function button(name: string): HTMLButtonElement {
  const element = [...host.querySelectorAll<HTMLButtonElement>('button')].find((b) => (b.getAttribute('aria-label') ?? b.textContent?.trim()) === name);
  if (!element) throw new Error(`Button not found: ${name}`);
  return element;
}

function menuItem(name: string): HTMLButtonElement {
  const element = [...host.querySelectorAll<HTMLButtonElement>('[role^="menuitem"]')].find((b) => b.querySelector('span')?.textContent === name);
  if (!element) throw new Error(`Menu item not found: ${name}`);
  return element;
}

async function click(element: HTMLElement) {
  element.click();
  await tick();
}

async function doubleClick(name: string) {
  button(name).dispatchEvent(new MouseEvent('dblclick', { bubbles: true }));
  await tick();
}

async function contextMenu(name: string) {
  button(name).dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true, clientX: 100, clientY: 100 }));
  await tick();
}

async function type(selector: string, value: string) {
  const input = host.querySelector<HTMLInputElement>(selector)!;
  expect(input).not.toBeNull();
  input.value = value;
  input.dispatchEvent(new Event('input', { bubbles: true }));
  await tick();
}

async function submitName(value: string) {
  await type('#entry-name', value);
  host.querySelector('.modal form')!.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }));
  await tick();
}

beforeEach(async () => {
  localStorage.clear();
  host = document.createElement('div');
  document.body.append(host);
  app = mount(App, { target: host });
  flushSync();
  await vi.waitFor(() => expect(host.querySelector('h1')?.textContent).toBe('主文件夹.'));
  await vi.waitFor(() => expect(host.querySelectorAll('.file-card')).toHaveLength(10));
});

afterEach(async () => {
  await unmount(app);
  host.remove();
  vi.restoreAllMocks();
});

describe('file manager interactions', () => {
  it('previews images in the inspector and dialog without launching an external app', async () => {
    const open = vi.spyOn(api, 'open');
    await doubleClick('Pictures');
    await vi.waitFor(() => expect(button('海岸.svg')).toBeTruthy());
    await click(button('海岸.svg'));
    await vi.waitFor(() => expect(host.querySelector('.inspector-image img')?.getAttribute('alt')).toBe('海岸.svg'));
    await click(button('预览图片'));
    expect(host.querySelector('.image-modal[open]')).not.toBeNull();
    expect(host.querySelector('#image-preview-title')?.textContent).toBe('海岸.svg');
    expect(button('上一张图片').disabled).toBe(true);
    await click(button('下一张图片'));
    expect(host.querySelector('#image-preview-title')?.textContent).toBe('山间.svg');
    expect(button('下一张图片').disabled).toBe(true);
    await click(button('实际大小'));
    expect(host.querySelector('.image-modal .image-preview.actual-size')).not.toBeNull();
    host.querySelector('.image-modal')!.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowLeft', bubbles: true }));
    await tick();
    expect(host.querySelector('#image-preview-title')?.textContent).toBe('海岸.svg');
    expect(host.querySelector('.image-modal .actual-size')).toBeNull();
    await click(button('关闭图片预览'));
    expect(host.querySelector('.image-modal')).toBeNull();
    await doubleClick('海岸.svg');
    expect(host.querySelector('.image-modal[open]')).not.toBeNull();
    await click(button('关闭图片预览'));
    button('海岸.svg').dispatchEvent(new KeyboardEvent('keydown', { key: ' ', code: 'Space', bubbles: true, cancelable: true }));
    await tick();
    expect(host.querySelector('.image-modal[open]')).not.toBeNull();
    expect(open).not.toHaveBeenCalled();
  });

  it('shows image load and decoding failures and clears preview after changing selection', async () => {
    vi.spyOn(api, 'image').mockRejectedValueOnce(new Error('图片超过 32 MB，请使用默认应用打开'));
    await doubleClick('Pictures');
    await vi.waitFor(() => expect(button('海岸.svg')).toBeTruthy());
    await click(button('海岸.svg'));
    await vi.waitFor(() => expect(host.querySelector('.inspector-image')?.textContent).toContain('图片超过 32 MB'));
    await click(button('山间.svg'));
    await vi.waitFor(() => expect(host.querySelector('.inspector-image img')).not.toBeNull());
    host.querySelector('.inspector-image img')!.dispatchEvent(new Event('error'));
    await tick();
    expect(host.querySelector('.inspector-image')?.textContent).toContain('无法预览图片');
    await click(button('旅行'));
    expect(host.querySelector('.inspector-image')).toBeNull();
  });

  it('discards stale image responses and releases image URLs when selection changes', async () => {
    const stale = URL.createObjectURL(new Blob(['stale']));
    const current = URL.createObjectURL(new Blob(['current']));
    let resolveImage!: (url: string) => void;
    const pending = new Promise<string>((resolve) => resolveImage = resolve);
    vi.spyOn(api, 'image').mockReturnValueOnce(pending).mockResolvedValueOnce(current);
    const revoke = vi.spyOn(URL, 'revokeObjectURL');
    await doubleClick('Pictures');
    await vi.waitFor(() => expect(button('海岸.svg')).toBeTruthy());
    await click(button('海岸.svg'));
    await click(button('山间.svg'));
    await vi.waitFor(() => expect(host.querySelector('.inspector-image img')?.getAttribute('src')).toBe(current));
    resolveImage(stale);
    await vi.waitFor(() => expect(revoke).toHaveBeenCalledWith(stale));
    expect(host.querySelector('.inspector-image img')?.getAttribute('src')).toBe(current);
    await click(button('旅行'));
    expect(revoke).toHaveBeenCalledWith(current);
  });

  it('navigates with history, parent controls and manually entered paths', async () => {
    await doubleClick('Documents');
    await vi.waitFor(() => expect(host.querySelector('h1')?.textContent).toBe('文档.'));
    await click(button('后退'));
    await vi.waitFor(() => expect(host.querySelector('h1')?.textContent).toBe('主文件夹.'));
    await click(button('前进'));
    await vi.waitFor(() => expect(host.querySelector('h1')?.textContent).toBe('文档.'));
    await click(button('上一级'));
    await vi.waitFor(() => expect(host.querySelector('h1')?.textContent).toBe('主文件夹.'));
    await click(button('编辑路径'));
    await type('[aria-label="文件夹路径"]', '~/Downloads');
    host.querySelector('.address-form')!.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }));
    await vi.waitFor(() => expect(host.querySelector('h1')?.textContent).toBe('下载.'));
    expect(button('设计素材.zip')).toBeTruthy();
  });

  it('filters, reveals hidden entries and persists view preferences', async () => {
    await click(button('更多显示选项'));
    await click(menuItem('显示隐藏文件'));
    expect(host.querySelectorAll('.file-card')).toHaveLength(11);
    expect(button('.config')).toBeTruthy();
    await type('[aria-label="搜索当前文件夹"]', 'document');
    expect(host.querySelectorAll('.file-card')).toHaveLength(1);
    await click(button('Documents'));
    expect(host.querySelector('.inspector-name h2')?.textContent).toBe('Documents');
    await type('[aria-label="搜索当前文件夹"]', 'not-found');
    expect(host.querySelectorAll('[aria-pressed="true"].file-card')).toHaveLength(0);
    expect(host.querySelector('.empty-state h2')?.textContent).toBe('没有找到匹配的项目');
    await click(button('清除搜索'));
    await click(button('列表视图'));
    expect(host.querySelectorAll('.file-row')).toHaveLength(11);
    await click(button('切换深色外观'));
    expect(host.querySelector('.app-shell')?.getAttribute('data-theme')).toBe('dark');
    expect(JSON.parse(localStorage.getItem('pudding.preferences')!)).toMatchObject({ hidden: true, view: 'list', dark: true });
  });

  it('creates, validates, renames, copies, cuts and trashes entries through the UI', async () => {
    await click(button('新建文件夹'));
    await submitName('集成测试文件夹');
    await vi.waitFor(() => expect(host.querySelector('dialog[open]')).toBeNull());
    await contextMenu('集成测试文件夹');
    await click(menuItem('重命名'));
    await submitName('Documents');
    await vi.waitFor(() => expect(host.querySelector('.field-error')?.textContent).toContain('同名'));
    await submitName('重命名结果');
    await vi.waitFor(() => expect(host.querySelector('dialog[open]')).toBeNull());
    await contextMenu('重命名结果');
    await click(menuItem('复制'));
    await doubleClick('Documents');
    await vi.waitFor(() => expect(host.querySelector('h1')?.textContent).toBe('文档.'));
    await click(host.querySelector<HTMLButtonElement>('.clipboard-strip > button')!);
    await vi.waitFor(() => expect(button('重命名结果')).toBeTruthy());
    await click(host.querySelector<HTMLButtonElement>('.clipboard-strip > button')!);
    await vi.waitFor(() => expect(host.querySelector('[role="alert"]')?.textContent).toContain('同名'));
    await contextMenu('重命名结果');
    await click(menuItem('剪切'));
    await click(button('上一级'));
    await vi.waitFor(() => expect(host.querySelector('h1')?.textContent).toBe('主文件夹.'));
    await doubleClick('Downloads');
    await vi.waitFor(() => expect(host.querySelector('h1')?.textContent).toBe('下载.'));
    await click(host.querySelector<HTMLButtonElement>('.clipboard-strip > button')!);
    await vi.waitFor(() => expect(button('重命名结果')).toBeTruthy());
    expect(host.querySelector('.clipboard-strip')).toBeNull();
    await contextMenu('重命名结果');
    await click(menuItem('移入回收站'));
    expect(host.querySelector('.trash-modal[open]')).not.toBeNull();
    await click(host.querySelector<HTMLButtonElement>('.trash-modal .danger-button')!);
    await vi.waitFor(() => expect(host.querySelector('.trash-modal[open]')).toBeNull());
    expect([...host.querySelectorAll('.file-card')].some((b) => b.getAttribute('aria-label') === '重命名结果')).toBe(false);
    await click(button('上一级'));
    await vi.waitFor(() => expect(host.querySelector('h1')?.textContent).toBe('主文件夹.'));
    await doubleClick('Documents');
    await vi.waitFor(() => expect(host.querySelector('h1')?.textContent).toBe('文档.'));
    expect([...host.querySelectorAll('.file-card')].some((b) => b.getAttribute('aria-label') === '重命名结果')).toBe(false);
  });
});
