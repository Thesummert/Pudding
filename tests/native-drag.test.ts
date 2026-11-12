import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { flushSync, mount, tick, unmount } from 'svelte';
import App from '../src/App.svelte';
import { api } from '../src/lib/api';

// Exercise the desktop pointer gesture without starting a real OS drag session.
vi.mock('../src/lib/api', async (original) => {
  const module = await original<typeof import('../src/lib/api')>();
  return { ...module, desktop: true, api: { ...module.api, startDrag: vi.fn().mockResolvedValue(undefined) } };
});

let host: HTMLDivElement;
let app: ReturnType<typeof mount>;

function file(name: string) {
  return host.querySelector<HTMLButtonElement>(`.file-card[aria-label="${name}"]`)!;
}

function pointer(target: EventTarget, type: string, x: number, y: number, buttons = 1) {
  target.dispatchEvent(new PointerEvent(type, { bubbles: true, pointerId: 1, button: 0, buttons, clientX: x, clientY: y }));
}

beforeEach(async () => {
  localStorage.clear();
  vi.mocked(api.startDrag).mockClear();
  host = document.createElement('div');
  document.body.append(host);
  app = mount(App, { target: host });
  flushSync();
  await vi.waitFor(() => expect(file('Documents')).toBeTruthy());
});

afterEach(async () => {
  await unmount(app);
  host.remove();
});

it('starts a native drag only after the movement threshold and preserves multiple selection', async () => {
  file('Documents').click();
  file('Downloads').dispatchEvent(new MouseEvent('click', { bubbles: true, ctrlKey: true }));
  await tick();
  pointer(file('Documents'), 'pointerdown', 20, 20);
  pointer(window, 'pointermove', 22, 21);
  expect(api.startDrag).not.toHaveBeenCalled();
  pointer(window, 'pointermove', 28, 20);
  expect(api.startDrag).toHaveBeenCalledWith(['/home/pudding/Documents', '/home/pudding/Downloads']);
  pointer(window, 'pointermove', 30, 20);
  expect(api.startDrag).toHaveBeenCalledOnce();
  expect(file('Documents').getAttribute('draggable')).toBe('false');
});

it('drags an unselected item alone and cancels pending gestures on release or blur', async () => {
  file('Documents').click();
  pointer(file('Downloads'), 'pointerdown', 20, 20);
  pointer(window, 'pointerup', 20, 20, 0);
  pointer(window, 'pointermove', 30, 20);
  expect(api.startDrag).not.toHaveBeenCalled();
  pointer(file('Downloads'), 'pointerdown', 20, 20);
  window.dispatchEvent(new Event('blur'));
  pointer(window, 'pointermove', 30, 20);
  expect(api.startDrag).not.toHaveBeenCalled();
  pointer(file('Downloads'), 'pointerdown', 20, 20);
  pointer(window, 'pointermove', 30, 20);
  expect(api.startDrag).toHaveBeenCalledWith(['/home/pudding/Downloads']);
});
