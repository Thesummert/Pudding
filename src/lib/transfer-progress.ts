import { formatSize } from './files';

export function transferEstimate(done: number, total: number | null, startedAt: number | null, now: number, mode: 'copy' | 'cut'): { speed: string; remaining: string } {
  const elapsed = startedAt === null ? 0 : (now - startedAt) / 1000;
  const rate = elapsed > 0 && done > 0 ? done / elapsed : 0;
  const speed = rate > 0 ? (mode === 'copy' ? `${rate < 1 ? `${rate.toFixed(1)} B` : formatSize(rate)}/秒` : `${rate.toFixed(1)} 项/秒`) : '计算中…';
  if (!rate || total === null || total <= 0) return { speed, remaining: '预计剩余时间：计算中…' };
  const seconds = Math.ceil(Math.max(0, total - done) / rate);
  const minutes = Math.ceil(seconds / 60);
  const duration = seconds < 60 ? `${seconds} 秒` : minutes < 60 ? `${minutes} 分钟` : `${Math.floor(minutes / 60)} 小时 ${minutes % 60} 分钟`;
  return { speed, remaining: `预计剩余 ${duration}` };
}
