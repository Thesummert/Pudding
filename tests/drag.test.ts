import { describe, expect, it } from 'vitest';
import { dragClientPosition } from '../src/lib/drag';

describe('native drag coordinates', () => {
  it.each(['macos', 'linux'])('keeps %s logical coordinates unchanged on high DPI screens', (platform) => {
    for (const scale of [1, 1.5, 2]) {
      expect(dragClientPosition({ x: 480, y: 360 }, platform, scale)).toEqual({ x: 480, y: 360 });
    }
  });

  it('converts Windows physical client pixels once', () => {
    for (const scale of [1, 1.5, 2]) {
      expect(dragClientPosition({ x: 480 * scale, y: 360 * scale }, 'windows', scale)).toEqual({ x: 480, y: 360 });
    }
  });
});
