import { beforeEach, describe, expect, it } from 'vitest';
import { defaultShortcuts, loadShortcuts, shortcutFromEvent } from '../src/lib/shortcuts';

describe('shortcuts', () => {
  beforeEach(() => localStorage.clear());
  it('loads saved bindings and falls back for invalid values', () => {
    localStorage.setItem('pudding.shortcuts', JSON.stringify({ search: 'Alt+KeyS', copy: 'invalid' }));
    expect(loadShortcuts()).toMatchObject({ search: 'Alt+KeyS', copy: defaultShortcuts().copy });
  });
  it('rejects conflicting saved bindings', () => {
    localStorage.setItem('pudding.shortcuts', JSON.stringify({ search: defaultShortcuts().copy }));
    expect(loadShortcuts()).toEqual(defaultShortcuts());
  });
  it('records physical keys and modifier combinations', () => {
    expect(shortcutFromEvent(new KeyboardEvent('keydown', { code: 'KeyN', altKey: true, shiftKey: true }))).toBe('Alt+Shift+KeyN');
    expect(shortcutFromEvent(new KeyboardEvent('keydown', { code: 'ShiftLeft', shiftKey: true }))).toBeNull();
  });
});
