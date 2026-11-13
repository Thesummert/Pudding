import { describe, expect, it } from 'vitest';
import { transferEstimate } from '../src/lib/transfer-progress';

describe('transfer estimates', () => {
  it('waits for progress and a known total before estimating time', () => {
    expect(transferEstimate(0, null, null, 1000, 'copy')).toEqual({ speed: '计算中…', remaining: '预计剩余时间：计算中…' });
    expect(transferEstimate(1024, null, 1000, 2000, 'copy')).toEqual({ speed: '1 KB/秒', remaining: '预计剩余时间：计算中…' });
  });

  it('estimates copy speed and remaining time from transferred bytes', () => {
    expect(transferEstimate(1024, 4096, 1000, 2000, 'copy')).toEqual({ speed: '1 KB/秒', remaining: '预计剩余 3 秒' });
    expect(transferEstimate(1024, 4096, 1000, 3000, 'copy')).toEqual({ speed: '512 B/秒', remaining: '预计剩余 6 秒' });
  });

  it('uses item counts for moves and formats longer estimates', () => {
    expect(transferEstimate(1, 121, 0, 1000, 'cut')).toEqual({ speed: '1.0 项/秒', remaining: '预计剩余 2 分钟' });
    expect(transferEstimate(1, 3602, 0, 1000, 'cut').remaining).toBe('预计剩余 1 小时 1 分钟');
  });
});
