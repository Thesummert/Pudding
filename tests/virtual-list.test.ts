import { describe, expect, it } from 'vitest';
import { virtualWindow } from '../src/lib/virtual-list';

describe('large directory rendering window', () => {
  it('keeps grid rows aligned and bounds the rendered entries when scrolling', () => {
    const first = virtualWindow(10000, 'grid', 120, 640, 500, 0, 21);
    expect(first.columns).toBe(5);
    expect(first.start).toBe(0);
    expect(first.end).toBeLessThan(100);
    const middle = virtualWindow(10000, 'grid', 120, 640, 500, 40000, 21);
    expect(middle.start % 5).toBe(0);
    expect(middle.before + 16).toBe(middle.start / 5 * middle.rowStep);
    expect(middle.end - middle.start).toBeLessThan(100);
    const last = virtualWindow(10000, 'grid', 120, 640, 500, 999999, 21);
    expect(last.end).toBe(10000);
    expect(last.after).toBe(0);
  });

  it('retains list offsets and does not virtualize small or unmeasured lists', () => {
    const list = virtualWindow(10000, 'list', 120, 600, 400, 4300, 0);
    expect(list.columns).toBe(1);
    expect(list.before).toBe(list.start * 43);
    expect(list.end - list.start).toBeLessThan(25);
    expect(virtualWindow(300, 'list', 120, 600, 400, 0, 0).end).toBe(300);
    expect(virtualWindow(1000, 'grid', 120, 0, 0, 0, 0).end).toBe(1000);
  });
});
