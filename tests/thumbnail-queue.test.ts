import { expect, it, vi } from 'vitest';
import { queueThumbnail } from '../src/lib/thumbnail-queue';

it('limits concurrent reads and skips thumbnails that leave view while queued', async () => {
  const complete: Array<(value: number) => void> = [];
  const load = vi.fn(() => new Promise<number>((resolve) => complete.push(resolve)));
  let visible = true;
  const first = queueThumbnail(load, () => true);
  const second = queueThumbnail(load, () => true);
  const skipped = queueThumbnail(load, () => visible);
  const last = queueThumbnail(load, () => true);
  await Promise.resolve();
  expect(load).toHaveBeenCalledTimes(2);
  visible = false;
  complete[0](1);
  expect(await first).toBe(1);
  expect(await skipped).toBeNull();
  await vi.waitFor(() => expect(load).toHaveBeenCalledTimes(3));
  complete[1](2);
  complete[2](3);
  expect(await second).toBe(2);
  expect(await last).toBe(3);
});
