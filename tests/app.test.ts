import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { flushSync, mount, tick, unmount } from 'svelte';
import App from '../src/App.svelte';
import { api } from '../src/lib/api';
import type { DragDropEvent } from '@tauri-apps/api/webview';
import { PhysicalPosition } from '@tauri-apps/api/dpi';

let app: ReturnType<typeof mount>;
let host: HTMLDivElement;
let nativeDrop: (event: DragDropEvent) => void;
let stopDrop: ReturnType<typeof vi.fn>;

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
  const clipboard = await api.readClipboard();
  if (clipboard) await api.clearClipboard(clipboard);
  stopDrop = vi.fn();
  vi.spyOn(api, 'listenDrop').mockImplementation(async (handler) => {
    nativeDrop = handler;
    return stopDrop;
  });
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
  vi.unstubAllGlobals();
});

describe('file manager interactions', () => {
  it('pins selected and current folders, restores them, navigates and removes them', async () => {
    await click(button('Documents'));
    await click(button('固定当前或选中的文件夹'));
    expect(JSON.parse(localStorage.getItem('pudding.pinnedFolders')!)).toEqual(['/home/pudding/Documents']);
    expect(host.querySelector('nav[aria-label="常用位置"]')?.contains(button('打开固定文件夹 Documents'))).toBe(true);
    expect(button('打开固定文件夹 Documents').dataset.dropPath).toBe('/home/pudding/Documents');
    expect(button('固定当前或选中的文件夹').disabled).toBe(true);
    await doubleClick('Documents');
    await vi.waitFor(() => expect(host.querySelector('h1')?.textContent).toBe('文档.'));
    await click(button('上一级'));
    await vi.waitFor(() => expect(host.querySelector('h1')?.textContent).toBe('主文件夹.'));
    await click(button('固定当前或选中的文件夹'));
    await unmount(app);
    app = mount(App, { target: host });
    await vi.waitFor(() => expect(button('打开固定文件夹 Documents')).toBeTruthy());
    await vi.waitFor(() => expect(button('打开固定文件夹 Documents').disabled).toBe(false));
    expect(button('打开固定文件夹 pudding')).toBeTruthy();
    await click(button('打开固定文件夹 Documents'));
    await vi.waitFor(() => expect(host.querySelector('h1')?.textContent).toBe('文档.'));
    await click(button('取消固定 Documents'));
    expect(JSON.parse(localStorage.getItem('pudding.pinnedFolders')!)).toEqual(['/home/pudding']);
  });

  it.each(['grid', 'list'])('selects files with a marquee in %s view and supports additive selection', async (view) => {
    if (view === 'list') await click(button('列表视图'));
    const area = host.querySelector<HTMLElement>('.file-area')!;
    vi.spyOn(area, 'getBoundingClientRect').mockReturnValue(new DOMRect(0, 0, 500, 500));
    const cards = [...host.querySelectorAll<HTMLElement>(view === 'grid' ? '.file-card' : '.file-row')];
    cards.forEach((card, index) => vi.spyOn(card, 'getBoundingClientRect').mockReturnValue(new DOMRect(30 + index * 45, 30, 30, 30)));
    function pointer(target: EventTarget, type: string, x: number, y: number, options: { ctrlKey?: boolean; buttons?: number } = {}) {
      target.dispatchEvent(new PointerEvent(type, { bubbles: true, pointerId: 7, button: 0, buttons: options.buttons ?? 1, clientX: x, clientY: y, ctrlKey: options.ctrlKey }));
    }
    pointer(area, 'pointerdown', 10, 10);
    pointer(window, 'pointermove', 110, 70);
    await tick();
    expect(host.querySelector('.selection-marquee')).not.toBeNull();
    expect(cards.filter((card) => card.getAttribute('aria-pressed') === 'true')).toHaveLength(2);
    pointer(window, 'pointerup', 110, 70, { buttons: 0 });
    await tick();
    expect(host.querySelector('.selection-marquee')).toBeNull();
    pointer(area, 'pointerdown', 115, 10, { ctrlKey: true });
    pointer(window, 'pointermove', 155, 70, { ctrlKey: true });
    pointer(window, 'pointerup', 155, 70, { buttons: 0, ctrlKey: true });
    await tick();
    expect(cards.filter((card) => card.getAttribute('aria-pressed') === 'true')).toHaveLength(3);
    const transfer = vi.spyOn(api, 'writeClipboard').mockResolvedValue();
    cards[0].dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true }));
    await tick();
    await click(menuItem('复制'));
    expect(transfer).toHaveBeenCalledWith({ paths: cards.slice(0, 3).map((card) => `/home/pudding/${card.getAttribute('aria-label')}`), mode: 'copy' });
    pointer(area, 'pointerdown', 10, 10);
    pointer(window, 'pointerup', 10, 10, { buttons: 0 });
    await tick();
    expect(cards.filter((card) => card.getAttribute('aria-pressed') === 'true')).toHaveLength(0);
  });

  it('opens requested directories at startup and while running', async () => {
    await unmount(app);
    let onOpen!: () => void;
    const stopOpen = vi.fn();
    vi.spyOn(api, 'listenOpenDirectories').mockImplementation(async (handler) => { onOpen = handler; return stopOpen; });
    const take = vi.spyOn(api, 'takeOpenDirectories').mockResolvedValueOnce(['/home/pudding/Documents']).mockResolvedValue([]);
    const openWindow = vi.spyOn(api, 'openWindow').mockResolvedValue();
    app = mount(App, { target: host });
    await vi.waitFor(() => expect(host.querySelector('h1')?.textContent).toBe('文档.'));
    take.mockResolvedValueOnce(['/home/pudding/Downloads', '/home/pudding/Pictures']);
    onOpen();
    await vi.waitFor(() => expect(host.querySelector('h1')?.textContent).toBe('下载.'));
    expect(openWindow).toHaveBeenCalledWith('/home/pudding/Pictures');
    take.mockResolvedValueOnce(['/home/pudding/missing']);
    onOpen();
    await vi.waitFor(() => expect(host.querySelector('[role="alert"]')).not.toBeNull());
    expect(host.querySelector('h1')?.textContent).toBe('下载.');
    await unmount(app);
    expect(stopOpen).toHaveBeenCalledOnce();
    app = mount(App, { target: host });
    await tick();
  });

  it('restores the startup directory and records the active directory', async () => {
    const startup = vi.spyOn(api, 'startupDirectory').mockResolvedValue('/home/pudding/Documents');
    const record = vi.spyOn(api, 'setWindowDirectory').mockResolvedValue();
    await unmount(app);
    app = mount(App, { target: host });
    await vi.waitFor(() => expect(host.querySelector('h1')?.textContent).toBe('文档.'));
    expect(record).toHaveBeenCalledWith('/home/pudding/Documents');
    await click(button('上一级'));
    await vi.waitFor(() => expect(record).toHaveBeenCalledWith('/home/pudding'));
    startup.mockResolvedValue(null);
    await unmount(app);
    app = mount(App, { target: host });
    await vi.waitFor(() => expect(host.querySelector('h1')?.textContent).toBe('主文件夹.'));
  });

  it('keeps Retina macOS drops in the middle pane instead of hitting the sidebar', async () => {
    const info = await api.info();
    vi.spyOn(api, 'info').mockResolvedValue({ ...info, platform: 'macos' });
    vi.stubGlobal('devicePixelRatio', 2);
    await unmount(app);
    app = mount(App, { target: host });
    flushSync();
    await vi.waitFor(() => expect(button('Documents')).toBeTruthy());
    expect(window.devicePixelRatio).toBe(2);
    vi.spyOn(api, 'transferConflicts').mockResolvedValue([]);
    const transfer = vi.spyOn(api, 'transfer').mockResolvedValue([]);
    // The old extra division mapped x=400 to x=200 in the sidebar.
    const hit = vi.spyOn(document, 'elementFromPoint').mockImplementation((x, y) => {
      if (x < 220) return host.querySelector('.places button');
      return y > 350 ? button('Documents').querySelector('span') : host.querySelector('.file-area');
    });
    nativeDrop({ type: 'enter', position: new PhysicalPosition(400, 300), paths: ['/tmp/空白落点.txt'] });
    await tick();
    expect(host.querySelector('.file-area.drop-target')).not.toBeNull();
    expect(host.querySelector('.places .drop-target')).toBeNull();
    nativeDrop({ type: 'drop', position: new PhysicalPosition(400, 300), paths: ['/tmp/空白落点.txt'] });
    await vi.waitFor(() => expect(host.querySelector('.drop-menu')).not.toBeNull());
    await click(menuItem('复制到此处'));
    await vi.waitFor(() => expect(transfer).toHaveBeenCalledWith(['/tmp/空白落点.txt'], '/home/pudding', 'copy'));
    await vi.waitFor(() => expect(host.querySelector('.file-area')?.getAttribute('aria-busy')).toBe('false'));
    nativeDrop({ type: 'drop', position: new PhysicalPosition(450, 400), paths: ['/tmp/文件夹落点.txt'] });
    await vi.waitFor(() => expect(host.querySelector('.drop-menu')).not.toBeNull());
    await click(menuItem('复制到此处'));
    await vi.waitFor(() => expect(transfer).toHaveBeenCalledWith(['/tmp/文件夹落点.txt'], '/home/pudding/Documents', 'copy'));
    expect(hit).toHaveBeenCalledWith(400, 300);
    expect(hit).toHaveBeenCalledWith(450, 400);
    await vi.waitFor(() => expect(host.querySelector('.file-area')?.getAttribute('aria-busy')).toBe('false'));
  });

  it('offers cancel, keep both and replace for clipboard conflicts', async () => {
    await api.writeClipboard({ paths: ['/home/pudding/开始使用.md'], mode: 'cut' });
    window.dispatchEvent(new Event('focus'));
    await vi.waitFor(() => expect(host.querySelector('.clipboard-strip')).not.toBeNull());
    const pasteButton = () => host.querySelector<HTMLButtonElement>('.clipboard-strip > button')!;
    await click(pasteButton());
    await vi.waitFor(() => expect(host.querySelector('.conflict-modal[open]')).not.toBeNull());
    await click(button('取消'));
    await vi.waitFor(() => expect(host.querySelector('.conflict-modal')).toBeNull());
    expect(await api.readClipboard()).toEqual({ paths: ['/home/pudding/开始使用.md'], mode: 'cut' });
    await vi.waitFor(() => expect(host.querySelector('.file-area')?.getAttribute('aria-busy')).toBe('false'));
    await api.writeClipboard({ paths: ['/home/pudding/开始使用.md'], mode: 'copy' });
    await click(pasteButton());
    await vi.waitFor(() => expect(host.querySelector('.conflict-modal[open]')).not.toBeNull());
    await click(button('并存'));
    await vi.waitFor(() => expect(button('开始使用 副本.md')).toBeTruthy());
    await doubleClick('Documents');
    await vi.waitFor(() => expect(host.querySelector('h1')?.textContent).toBe('文档.'));
    await click(pasteButton());
    await vi.waitFor(() => expect(button('开始使用.md')).toBeTruthy());
    await click(pasteButton());
    await vi.waitFor(() => expect(host.querySelector('.conflict-modal[open]')).not.toBeNull());
    await click(button('替换'));
    await vi.waitFor(() => expect(host.querySelector('.conflict-modal')).toBeNull());
    await vi.waitFor(() => expect(host.querySelector('[role="status"]')?.textContent ?? host.textContent).toContain('已复制'));
    await api.delete(['/home/pudding/开始使用 副本.md', '/home/pudding/Documents/开始使用.md']);
  });

  it('renders only nearby items in large directories and keeps their original indices', async () => {
    const listing = await api.read('/home/pudding');
    const template = listing.entries.find((entry) => !entry.isDir)!;
    const entries = Array.from({ length: 2000 }, (_, i) => ({ ...template, name: `文件 ${String(i).padStart(4, '0')}.txt`, path: `/home/pudding/file-${i}.txt` }));
    vi.spyOn(api, 'read').mockResolvedValue({ ...listing, entries });
    const area = host.querySelector<HTMLDivElement>('.file-area')!;
    Object.defineProperties(area, { clientWidth: { configurable: true, value: 680 }, clientHeight: { configurable: true, value: 400 } });
    area.dispatchEvent(new Event('scroll'));
    await click(button('刷新'));
    await vi.waitFor(() => expect(host.textContent).toContain('2000 个项目'));
    expect(host.querySelectorAll('.file-card').length).toBeLessThan(100);
    area.scrollTop = 4000;
    area.dispatchEvent(new Event('scroll'));
    await vi.waitFor(() => expect(Number(host.querySelector('.file-card')?.getAttribute('data-index'))).toBeGreaterThan(0));
    const first = host.querySelector<HTMLButtonElement>('.file-card')!;
    await click(first);
    expect(first.getAttribute('aria-pressed')).toBe('true');
    await click(button('列表视图'));
    await vi.waitFor(() => expect(host.querySelectorAll('.file-row').length).toBeGreaterThan(0));
    expect(host.querySelectorAll('.file-row').length).toBeLessThan(30);
  });

  it('loads only visible pending metadata and replaces unknown sizes', async () => {
    const listing = await api.read('/home/pudding');
    const template = listing.entries.find((entry) => !entry.isDir)!;
    const entries = Array.from({ length: 1000 }, (_, i) => ({ ...template, name: `照片 ${String(i).padStart(4, '0')}.txt`, path: `/home/pudding/photo-${i}.txt`, size: 0, modified: null, metadataPending: true }));
    vi.spyOn(api, 'read').mockResolvedValue({ ...listing, entries });
    const details = vi.spyOn(api, 'readEntryDetails').mockImplementation(async (_path, names) => entries.filter((entry) => names.includes(entry.name)).map((entry) => ({ ...entry, size: 2048, modified: 1, metadataPending: false })));
    const area = host.querySelector<HTMLDivElement>('.file-area')!;
    Object.defineProperties(area, { clientWidth: { configurable: true, value: 680 }, clientHeight: { configurable: true, value: 400 } });
    area.dispatchEvent(new Event('scroll'));
    await click(button('刷新'));
    await vi.waitFor(() => expect(details).toHaveBeenCalled());
    await vi.waitFor(() => expect(host.querySelector('.file-card .file-caption')?.textContent).toBe('2 KB'));
    expect(details.mock.calls.flatMap((call) => call[1]).length).toBeLessThan(100);
    area.scrollTop = 8000;
    area.dispatchEvent(new Event('scroll'));
    await vi.waitFor(() => expect(details.mock.calls.length).toBeGreaterThan(1));
  });

  it('reads fresh clipboard contents at paste time and reports failed clipboard writes', async () => {
    await contextMenu('开始使用.md');
    vi.spyOn(api, 'writeClipboard').mockRejectedValueOnce(new Error('系统剪贴板不可用'));
    await click(menuItem('剪切'));
    await vi.waitFor(() => expect(host.querySelector('[role="alert"]')?.textContent).toContain('系统剪贴板不可用'));
    expect(host.querySelector('.clipboard-strip')).toBeNull();
    await contextMenu('开始使用.md');
    await click(menuItem('复制'));
    await vi.waitFor(() => expect(host.querySelector('.clipboard-strip')).not.toBeNull());
    await doubleClick('Documents');
    await vi.waitFor(() => expect(host.querySelector('h1')?.textContent).toBe('文档.'));
    vi.spyOn(api, 'readClipboard').mockResolvedValue({ paths: ['/home/pudding/灵感笔记.txt'], mode: 'copy' });
    const transfer = vi.spyOn(api, 'transfer');
    await click(host.querySelector<HTMLButtonElement>('.clipboard-strip > button')!);
    await vi.waitFor(() => expect(transfer).toHaveBeenCalledWith(['/home/pudding/灵感笔记.txt'], '/home/pudding/Documents', 'copy'));
    await vi.waitFor(() => expect(button('灵感笔记.txt')).toBeTruthy());
    expect([...host.querySelectorAll('.file-card')].some((b) => b.getAttribute('aria-label') === '开始使用.md')).toBe(false);
  });

  it('does not clear a newer clipboard after a partially failed cut', async () => {
    await api.writeClipboard({ paths: ['/home/pudding/开始使用.md'], mode: 'cut' });
    window.dispatchEvent(new Event('focus'));
    await vi.waitFor(() => expect(host.querySelector('.clipboard-strip')).not.toBeNull());
    await doubleClick('Downloads');
    await vi.waitFor(() => expect(host.querySelector('h1')?.textContent).toBe('下载.'));
    const newer = { paths: ['/home/pudding/灵感笔记.txt'], mode: 'copy' as const };
    vi.spyOn(api, 'transfer').mockImplementationOnce(async () => {
      await api.writeClipboard(newer);
      throw new Error('已完成 1 个项目');
    });
    await click(host.querySelector<HTMLButtonElement>('.clipboard-strip > button')!);
    await vi.waitFor(() => expect(host.querySelector('[role="alert"]')?.textContent).toContain('已完成 1 个项目'));
    expect(await api.readClipboard()).toEqual(newer);
    await vi.waitFor(() => expect(host.querySelector('.clipboard-strip')?.textContent).toContain('等待复制'));
  });

  it.each(['grid', 'list'])('routes native drops to folders, places and blank space in %s view', async (view) => {
    if (view === 'list') await click(button('列表视图'));
    vi.spyOn(api, 'transferConflicts').mockResolvedValue([]);
    const transfer = vi.spyOn(api, 'transfer').mockResolvedValue([]);
    const hit = vi.spyOn(document, 'elementFromPoint').mockReturnValue(button('Documents').querySelector('span'));
    const position = new PhysicalPosition(200, 240);
    nativeDrop({ type: 'enter', position, paths: ['/tmp/外部文件.txt'] });
    await tick();
    expect(button('Documents').classList.contains('drop-target')).toBe(true);
    expect(hit).toHaveBeenCalledWith(200 / window.devicePixelRatio, 240 / window.devicePixelRatio);
    nativeDrop({ type: 'drop', position, paths: ['/tmp/外部文件.txt'] });
    await vi.waitFor(() => expect(host.querySelector('.drop-menu')).not.toBeNull());
    await click(menuItem('复制到此处'));
    await vi.waitFor(() => expect(transfer).toHaveBeenCalledWith(['/tmp/外部文件.txt'], '/home/pudding/Documents', 'copy'));
    await vi.waitFor(() => expect(host.querySelector('.file-area')?.getAttribute('aria-busy')).toBe('false'));
    expect(host.querySelector('.drop-hint')).toBeNull();
    hit.mockReturnValue(host.querySelector('[data-drop-path="/home/pudding/Downloads"]'));
    nativeDrop({ type: 'drop', position, paths: ['/tmp/第二项'] });
    await vi.waitFor(() => expect(host.querySelector('.drop-menu')).not.toBeNull());
    await click(menuItem('复制到此处'));
    await vi.waitFor(() => expect(transfer).toHaveBeenCalledWith(['/tmp/第二项'], '/home/pudding/Downloads', 'copy'));
    await vi.waitFor(() => expect(host.querySelector('.file-area')?.getAttribute('aria-busy')).toBe('false'));
    hit.mockReturnValue(button('开始使用.md'));
    nativeDrop({ type: 'drop', position, paths: ['/tmp/第三项'] });
    await vi.waitFor(() => expect(host.querySelector('.drop-menu')).not.toBeNull());
    await click(menuItem('复制到此处'));
    await vi.waitFor(() => expect(transfer).toHaveBeenCalledWith(['/tmp/第三项'], '/home/pudding', 'copy'));
    await vi.waitFor(() => expect(host.querySelector('.file-area')?.getAttribute('aria-busy')).toBe('false'));
    // Both whitespace between entries and the surrounding middle pane use
    // the currently open directory, while a folder's children take priority.
    for (const selector of ['.file-area', '.folder-heading']) {
      hit.mockReturnValue(host.querySelector(selector));
      nativeDrop({ type: 'over', position });
      await tick();
      expect(host.querySelector('.file-area')?.classList.contains('drop-target')).toBe(true);
      nativeDrop({ type: 'drop', position, paths: [`/tmp/${selector}`] });
      await vi.waitFor(() => expect(host.querySelector('.drop-menu')).not.toBeNull());
      await click(menuItem('复制到此处'));
      await vi.waitFor(() => expect(transfer).toHaveBeenCalledWith([`/tmp/${selector}`], '/home/pudding', 'copy'));
      await vi.waitFor(() => expect(host.querySelector('.file-area')?.getAttribute('aria-busy')).toBe('false'));
    }
    hit.mockReturnValue(host.querySelector('.app-header'));
    nativeDrop({ type: 'drop', position, paths: ['/tmp/忽略'] });
    expect(transfer).toHaveBeenCalledTimes(5);
    nativeDrop({ type: 'leave' });
    await tick();
    expect(host.querySelector('.drop-target')).toBeNull();
  });

  it('offers drop actions and uses modifiers for immediate transfers', async () => {
    vi.spyOn(api, 'transferConflicts').mockResolvedValue([]);
    const transfer = vi.spyOn(api, 'transfer').mockResolvedValue([]);
    vi.spyOn(document, 'elementFromPoint').mockReturnValue(host.querySelector('.file-area'));
    const position = new PhysicalPosition(120, 120);
    nativeDrop({ type: 'drop', position, paths: ['/tmp/move'] });
    await vi.waitFor(() => expect(host.querySelector('.drop-menu')).not.toBeNull());
    expect(transfer).not.toHaveBeenCalled();
    await click(menuItem('移动到此处'));
    await vi.waitFor(() => expect(transfer).toHaveBeenCalledWith(['/tmp/move'], '/home/pudding', 'cut'));
    await vi.waitFor(() => expect(host.querySelector('.file-area')?.getAttribute('aria-busy')).toBe('false'));
    vi.spyOn(api, 'dropModifiers').mockResolvedValue({ ctrl: true, shift: false });
    nativeDrop({ type: 'drop', position, paths: ['/tmp/copy'] });
    await vi.waitFor(() => expect(transfer).toHaveBeenCalledWith(['/tmp/copy'], '/home/pudding', 'copy'));
    await vi.waitFor(() => expect(host.querySelector('.file-area')?.getAttribute('aria-busy')).toBe('false'));
    vi.mocked(api.dropModifiers).mockResolvedValue({ ctrl: false, shift: true });
    nativeDrop({ type: 'drop', position, paths: ['/tmp/shift'] });
    await vi.waitFor(() => expect(transfer).toHaveBeenCalledWith(['/tmp/shift'], '/home/pudding', 'cut'));
  });

  it('removes the native drop listener on unmount', async () => {
    await unmount(app);
    expect(stopDrop).toHaveBeenCalledOnce();
    app = mount(App, { target: host });
    await tick();
  });

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

  it('opens and closes text and PDF previews with Space', async () => {
    const space = (target: Element) => target.dispatchEvent(new KeyboardEvent('keydown', { key: ' ', code: 'Space', bubbles: true, cancelable: true }));
    await click(button('开始使用.md'));
    space(button('开始使用.md'));
    await vi.waitFor(() => expect(host.querySelector('.text-preview')?.textContent).toContain('开始使用.md'));
    space(host.querySelector('.text-preview')!);
    await tick();
    expect(host.querySelector('.file-preview-modal')).toBeNull();
    await doubleClick('Documents');
    await vi.waitFor(() => expect(button('阅读清单.pdf')).toBeTruthy());
    vi.spyOn(api, 'previewBytes').mockReturnValue(new Promise(() => {}));
    await click(button('阅读清单.pdf'));
    space(button('阅读清单.pdf'));
    await vi.waitFor(() => expect(host.querySelector('.pdf-preview')).not.toBeNull());
    space(host.querySelector('.file-preview-modal')!);
    await tick();
    expect(host.querySelector('.file-preview-modal')).toBeNull();
  });

  it('shows basic information for folders and unsupported files with Space', async () => {
    vi.spyOn(api, 'directorySize').mockResolvedValue(292520);
    const space = (target: Element) => target.dispatchEvent(new KeyboardEvent('keydown', { key: ' ', code: 'Space', bubbles: true, cancelable: true }));
    await click(button('Documents'));
    space(button('Documents'));
    await vi.waitFor(() => expect(host.querySelector('.info-preview-modal')?.textContent).toContain('Documents'));
    await vi.waitFor(() => expect(host.querySelector('.info-preview-modal')?.textContent).toContain('285.7 KB'));
    expect(host.querySelector('.info-preview-modal')?.textContent).toContain('修改时间');
    space(host.querySelector('.info-preview-modal')!);
    await tick();
    expect(host.querySelector('.info-preview-modal')).toBeNull();
    await doubleClick('Downloads');
    await vi.waitFor(() => expect(button('设计素材.zip')).toBeTruthy());
    await click(button('设计素材.zip'));
    space(button('设计素材.zip'));
    await tick();
    expect(host.querySelector('.info-preview-modal')?.textContent).toContain('17.6 MB');
    expect(host.querySelector('.info-preview-modal')?.textContent).toContain('压缩文件');
    await click(button('关闭信息'));
    expect(host.querySelector('.info-preview-modal')).toBeNull();
  });

  it('opens a text preview from the context menu', async () => {
    await contextMenu('开始使用.md');
    await click(menuItem('预览'));
    await vi.waitFor(() => expect(host.querySelector('.text-preview')?.textContent).toContain('开始使用.md'));
    expect(host.querySelector('.context-menu')).toBeNull();
  });

  it('decodes UTF-16 text and reports binary content', async () => {
    const bytes = vi.spyOn(api, 'previewBytes').mockResolvedValueOnce(new Uint8Array([0xff, 0xfe, 0x41, 0x00, 0x42, 0x00])).mockResolvedValueOnce(new Uint8Array([0x41, 0x00, 0x42]));
    await click(button('开始使用.md'));
    const space = (target: Element) => target.dispatchEvent(new KeyboardEvent('keydown', { key: ' ', code: 'Space', bubbles: true, cancelable: true }));
    space(button('开始使用.md'));
    await vi.waitFor(() => expect(host.querySelector('.text-preview')?.textContent).toBe('AB'));
    space(host.querySelector('.text-preview')!);
    await tick();
    space(button('开始使用.md'));
    await vi.waitFor(() => expect(host.querySelector('.file-preview-message')?.textContent).toContain('二进制'));
    expect(bytes).toHaveBeenCalledTimes(2);
  });

  it('previews video and closes with Space from the player', async () => {
    const originalRead = api.read;
    vi.spyOn(api, 'read').mockImplementation(async (path) => {
      const listing = await originalRead(path);
      if (listing.path.endsWith('/Videos')) listing.entries.push({ name: '短片.mp4', path: `${listing.path}/短片.mp4`, extension: 'mp4', isDir: false, isSymlink: false, hidden: false, modified: 1, size: 123 });
      return listing;
    });
    vi.spyOn(api, 'videoSource').mockResolvedValue('blob:video-test');
    const play = vi.spyOn(HTMLMediaElement.prototype, 'play').mockRejectedValueOnce(new DOMException('autoplay blocked', 'NotAllowedError')).mockResolvedValueOnce();
    await doubleClick('Videos');
    await vi.waitFor(() => expect(button('短片.mp4')).toBeTruthy());
    await click(button('短片.mp4'));
    button('短片.mp4').dispatchEvent(new KeyboardEvent('keydown', { key: ' ', code: 'Space', bubbles: true, cancelable: true }));
    await vi.waitFor(() => expect(host.querySelector('.video-preview')).not.toBeNull());
    const player = host.querySelector<HTMLVideoElement>('.video-preview')!;
    expect(player.autoplay).toBe(true);
    player.dispatchEvent(new Event('loadedmetadata'));
    await vi.waitFor(() => expect(play).toHaveBeenCalledTimes(2));
    expect(player.muted).toBe(true);
    const event = new KeyboardEvent('keydown', { key: ' ', code: 'Space', bubbles: true, cancelable: true });
    host.querySelector('.video-preview')!.dispatchEvent(event);
    await tick();
    expect(event.defaultPrevented).toBe(true);
    expect(host.querySelector('.file-preview-modal')).toBeNull();
  });

  it('closes image preview with Space', async () => {
    await doubleClick('Pictures');
    await vi.waitFor(() => expect(button('海岸.svg')).toBeTruthy());
    await click(button('海岸.svg'));
    button('海岸.svg').dispatchEvent(new KeyboardEvent('keydown', { key: ' ', code: 'Space', bubbles: true, cancelable: true }));
    await tick();
    expect(host.querySelector('.image-modal[open]')).not.toBeNull();
    host.querySelector('.image-modal')!.dispatchEvent(new KeyboardEvent('keydown', { key: ' ', code: 'Space', bubbles: true, cancelable: true }));
    await tick();
    expect(host.querySelector('.image-modal')).toBeNull();
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

  it('opens a folder from the context menu in a tab or window', async () => {
    const openWindow = vi.spyOn(api, 'openWindow').mockResolvedValue();
    await contextMenu('Documents');
    await click(menuItem('在新窗口中打开'));
    expect(openWindow).toHaveBeenCalledWith('/home/pudding/Documents');
    await contextMenu('Documents');
    await click(menuItem('在新标签页中打开'));
    await vi.waitFor(() => expect(host.querySelectorAll('[role=tab]')).toHaveLength(2));
    await vi.waitFor(() => expect(host.querySelector('h1')?.textContent).toBe('文档.'));
  });

  it('keeps navigation history separate across tabs and closes the active tab', async () => {
    await doubleClick('Documents');
    await vi.waitFor(() => expect(host.querySelector('h1')?.textContent).toBe('文档.'));
    await click(button('新建标签页'));
    await vi.waitFor(() => expect(host.querySelectorAll('[role=tab]')).toHaveLength(2));
    await vi.waitFor(() => expect(button('上一级').disabled).toBe(false));
    await click(button('上一级'));
    await vi.waitFor(() => expect(host.querySelector('h1')?.textContent).toBe('主文件夹.'));
    await click(host.querySelectorAll<HTMLButtonElement>('[role=tab]')[0]);
    await vi.waitFor(() => expect(host.querySelector('h1')?.textContent).toBe('文档.'));
    await click(button('后退'));
    await vi.waitFor(() => expect(host.querySelector('h1')?.textContent).toBe('主文件夹.'));
    await click(button('前进'));
    await vi.waitFor(() => expect(host.querySelector('h1')?.textContent).toBe('文档.'));
    await click(host.querySelector<HTMLElement>('.tab.active .tab-close')!);
    await vi.waitFor(() => expect(host.querySelectorAll('[role=tab]')).toHaveLength(1));
    await vi.waitFor(() => expect(host.querySelector('h1')?.textContent).toBe('主文件夹.'));
  });

  it('refreshes connected volumes and safely ejects the current volume', async () => {
    const roots = [
      { id: '/', path: '/', name: '文件系统', ejectable: false },
      { id: 'disk', path: '/home/pudding/Documents', name: '移动磁盘', ejectable: true },
    ];
    const volumes = vi.spyOn(api, 'volumes').mockResolvedValue(roots);
    const eject = vi.spyOn(api, 'ejectVolume').mockResolvedValue();
    window.dispatchEvent(new Event('focus'));
    await vi.waitFor(() => expect(button('推出 移动磁盘')).toBeTruthy());
    expect(host.querySelector('[aria-label="推出 文件系统"]')).toBeNull();
    await doubleClick('Documents');
    await vi.waitFor(() => expect(host.querySelector('h1')?.textContent).toBe('文档.'));
    volumes.mockResolvedValue([roots[0]]);
    await click(button('推出 移动磁盘'));
    await vi.waitFor(() => expect(eject).toHaveBeenCalledWith('/home/pudding/Documents'));
    await vi.waitFor(() => expect(host.querySelector('h1')?.textContent).toBe('主文件夹.'));
    await vi.waitFor(() => expect(host.querySelector('[aria-label="推出 移动磁盘"]')).toBeNull());
    expect(button('后退').disabled).toBe(true);
  });

  it('keeps the current directory when ejecting a volume fails', async () => {
    vi.spyOn(api, 'volumes').mockResolvedValue([{ id: 'disk', path: '/home/pudding/Documents', name: '移动磁盘', ejectable: true }]);
    vi.spyOn(api, 'ejectVolume').mockRejectedValue(new Error('磁盘正在使用'));
    window.dispatchEvent(new Event('focus'));
    await vi.waitFor(() => expect(button('推出 移动磁盘')).toBeTruthy());
    await doubleClick('Documents');
    await vi.waitFor(() => expect(host.querySelector('h1')?.textContent).toBe('文档.'));
    await click(button('推出 移动磁盘'));
    await vi.waitFor(() => expect(host.querySelector('[role="alert"]')?.textContent).toContain('磁盘正在使用'));
    expect(host.querySelector('h1')?.textContent).toBe('文档.');
  });

  it('navigates the active tab with mouse side buttons and ignores them in dialogs', async () => {
    function sideButton(button: number, type = 'mousedown') {
      const event = new MouseEvent(type, { button, bubbles: true, cancelable: true });
      host.querySelector('.file-area')!.dispatchEvent(event);
      return event;
    }
    await doubleClick('Documents');
    await vi.waitFor(() => expect(host.querySelector('h1')?.textContent).toBe('文档.'));
    expect(sideButton(3).defaultPrevented).toBe(true);
    await vi.waitFor(() => expect(host.querySelector('h1')?.textContent).toBe('主文件夹.'));
    expect(sideButton(4).defaultPrevented).toBe(true);
    await vi.waitFor(() => expect(host.querySelector('h1')?.textContent).toBe('文档.'));
    expect(sideButton(3, 'auxclick').defaultPrevented).toBe(true);
    expect(sideButton(1).defaultPrevented).toBe(false);
    await click(button('新建文件夹'));
    expect(host.querySelector('dialog[open]')).not.toBeNull();
    expect(sideButton(3).defaultPrevented).toBe(true);
    await tick();
    expect(host.querySelector('h1')?.textContent).toBe('文档.');
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

  it('applies and saves settings from the settings dialog', async () => {
    await click(button('设置'));
    const settings = host.querySelector<HTMLDialogElement>('.settings-modal')!;
    expect(settings.open).toBe(true);
    const appearance = settings.querySelector<HTMLSelectElement>('[aria-label="外观"]')!;
    appearance.value = 'dark';
    appearance.dispatchEvent(new Event('change', { bubbles: true }));
    const hidden = settings.querySelector<HTMLInputElement>('input[type="checkbox"]')!;
    hidden.click();
    await tick();
    expect(host.querySelector('.app-shell')?.getAttribute('data-theme')).toBe('dark');
    expect(JSON.parse(localStorage.getItem('pudding.preferences')!)).toMatchObject({ dark: true, hidden: true });
    await click(button('完成'));
    expect(settings.open).toBe(false);
  });

  it('saves a chosen terminal application and uses it when opening a folder', async () => {
    vi.spyOn(api, 'chooseTerminal').mockResolvedValue('/Applications/My Terminal.app');
    const openTerminal = vi.spyOn(api, 'openTerminal').mockResolvedValue();
    await click(button('设置'));
    await click(button('选择应用…'));
    expect(JSON.parse(localStorage.getItem('pudding.preferences')!)).toMatchObject({ terminal: 'custom', customTerminal: '/Applications/My Terminal.app' });
    await click(button('完成'));
    await contextMenu('Documents');
    await click(menuItem('在终端中打开'));
    expect(openTerminal).toHaveBeenCalledWith('/home/pudding/Documents', 'custom', '/Applications/My Terminal.app');
  });

  it('opens a folder or the current directory in the configured terminal', async () => {
    const openTerminal = vi.spyOn(api, 'openTerminal').mockResolvedValue();
    await contextMenu('Documents');
    await click(menuItem('在终端中打开'));
    expect(openTerminal).toHaveBeenCalledWith('/home/pudding/Documents', 'system', '');
    host.querySelector('.file-area')!.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true }));
    await tick();
    await click(menuItem('在终端中打开'));
    expect(openTerminal).toHaveBeenLastCalledWith('/home/pudding', 'system', '');
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
    expect(JSON.parse(localStorage.getItem('pudding.preferences')!)).toMatchObject({ hidden: true, view: 'grid', dark: true });
    expect(JSON.parse(localStorage.getItem('pudding.folderDisplays')!)['/home/pudding'].view).toBe('list');
  });

  it('restores each folder’s display and sorting, and clears the records from settings', async () => {
    await click(button('列表视图'));
    const sort = host.querySelector<HTMLSelectElement>('[aria-label="排序方式"]')!;
    sort.value = 'size';
    sort.dispatchEvent(new Event('change', { bubbles: true }));
    await click(button('切换降序'));
    await doubleClick('Documents');
    await vi.waitFor(() => expect(host.querySelector('h1')?.textContent).toBe('文档.'));
    expect(button('图标视图').getAttribute('aria-pressed')).toBe('true');
    expect(sort.value).toBe('name');
    await click(button('上一级'));
    await vi.waitFor(() => expect(host.querySelector('h1')?.textContent).toBe('主文件夹.'));
    expect(button('列表视图').getAttribute('aria-pressed')).toBe('true');
    expect(sort.value).toBe('size');
    expect(button('切换升序')).toBeTruthy();
    await unmount(app);
    app = mount(App, { target: host });
    await vi.waitFor(() => expect(button('列表视图').getAttribute('aria-pressed')).toBe('true'));
    expect(host.querySelector<HTMLSelectElement>('[aria-label="排序方式"]')!.value).toBe('size');
    await click(button('设置'));
    await click(button('清理记录'));
    expect(localStorage.getItem('pudding.folderDisplays')).toBeNull();
    expect(button('图标视图').getAttribute('aria-pressed')).toBe('true');
    expect(host.querySelector<HTMLSelectElement>('[aria-label="排序方式"]')!.value).toBe('name');
    await click(button('完成'));
    await doubleClick('Documents');
    await vi.waitFor(() => expect(host.querySelector('h1')?.textContent).toBe('文档.'));
    await click(button('上一级'));
    await vi.waitFor(() => expect(host.querySelector('h1')?.textContent).toBe('主文件夹.'));
    expect(button('图标视图').getAttribute('aria-pressed')).toBe('true');
  });

  it('updates the context menu as Shift is pressed or released', async () => {
    await contextMenu('开始使用.md');
    expect(menuItem('移入回收站')).toBeTruthy();
    button('开始使用.md').dispatchEvent(new KeyboardEvent('keydown', { key: 'Shift', shiftKey: true, bubbles: true }));
    await tick();
    expect(menuItem('永久删除').textContent).toContain('Shift Del');
    button('开始使用.md').dispatchEvent(new KeyboardEvent('keyup', { key: 'Shift', bubbles: true }));
    await tick();
    expect(menuItem('移入回收站')).toBeTruthy();
    button('开始使用.md').dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true, shiftKey: true }));
    await tick();
    await click(menuItem('永久删除'));
    expect(host.querySelector('.trash-modal h2')?.textContent).toBe('永久删除？');
  });

  it('confirms permanent deletion with Shift and keeps ordinary deletion in the trash', async () => {
    const permanentlyDelete = vi.spyOn(api, 'delete').mockResolvedValue();
    const trash = vi.spyOn(api, 'trash').mockResolvedValue();
    await click(button('开始使用.md'));
    button('开始使用.md').dispatchEvent(new KeyboardEvent('keydown', { key: 'Delete', shiftKey: true, bubbles: true, cancelable: true }));
    await tick();
    expect(host.querySelector('.trash-modal h2')?.textContent).toBe('永久删除？');
    expect(permanentlyDelete).not.toHaveBeenCalled();
    await click(host.querySelector<HTMLButtonElement>('.trash-modal .danger-button')!);
    await vi.waitFor(() => expect(permanentlyDelete).toHaveBeenCalledWith(['/home/pudding/开始使用.md']));
    await vi.waitFor(() => expect(host.querySelector('.trash-modal[open]')).toBeNull());
    expect(trash).not.toHaveBeenCalled();
    await vi.waitFor(() => expect(host.querySelector('.file-area')?.getAttribute('aria-busy')).toBe('false'));
    await contextMenu('开始使用.md');
    menuItem('移入回收站').dispatchEvent(new MouseEvent('click', { bubbles: true, shiftKey: true }));
    await tick();
    expect(host.querySelector('.trash-modal h2')?.textContent).toBe('永久删除？');
    await click(host.querySelector<HTMLButtonElement>('.trash-modal .modal-actions .button')!);
    await contextMenu('开始使用.md');
    await click(menuItem('移入回收站'));
    expect(host.querySelector('.trash-modal h2')?.textContent).toBe('移入回收站？');
    await click(host.querySelector<HTMLButtonElement>('.trash-modal .danger-button')!);
    await vi.waitFor(() => expect(trash).toHaveBeenCalledWith(['/home/pudding/开始使用.md']));
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
    await vi.waitFor(() => expect(host.querySelector('.conflict-modal[open]')).not.toBeNull());
    await click(button('取消'));
    await vi.waitFor(() => expect(host.querySelector('.conflict-modal[open]')).toBeNull());
    await vi.waitFor(() => expect(host.querySelector('.file-area')?.getAttribute('aria-busy')).toBe('false'));
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
