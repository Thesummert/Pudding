import { api } from './api';

// Keep only small, encoded thumbnails; never retain original image buffers.
const maxCacheBytes = 24 * 1024 * 1024;
const maxEntries = 300;
const edge = 256;
const cache = new Map<string, Blob>();
let cacheBytes = 0;

function remember(key: string, blob: Blob) {
  if (blob.size > maxCacheBytes) return;
  const previous = cache.get(key);
  if (previous) cacheBytes -= previous.size;
  cache.delete(key);
  cache.set(key, blob);
  cacheBytes += blob.size;
  while (cache.size > maxEntries || cacheBytes > maxCacheBytes) {
    const oldest = cache.keys().next().value!;
    cacheBytes -= cache.get(oldest)!.size;
    cache.delete(oldest);
  }
}

export async function loadImageThumbnail(path: string, extension: string, modified: number | null, size: number, active: () => boolean): Promise<string | null> {
  const key = JSON.stringify([path, extension, modified, size]);
  const cached = cache.get(key);
  if (cached) {
    cache.delete(key);
    cache.set(key, cached);
    return active() ? URL.createObjectURL(cached) : null;
  }

  const original = await api.image(path, extension);
  if (!active()) { URL.revokeObjectURL(original); return null; }
  // Preserve browser support for formats that createImageBitmap cannot decode.
  if (typeof createImageBitmap !== 'function') return original;
  try {
    const blob = await (await fetch(original)).blob();
    if (!active()) { URL.revokeObjectURL(original); return null; }
    const bitmap = await createImageBitmap(blob);
    try {
      if (!active()) { URL.revokeObjectURL(original); return null; }
      const scale = Math.min(1, edge / Math.max(bitmap.width, bitmap.height));
      const canvas = document.createElement('canvas');
      canvas.width = Math.max(1, Math.round(bitmap.width * scale));
      canvas.height = Math.max(1, Math.round(bitmap.height * scale));
      const context = canvas.getContext('2d');
      if (!context) return original;
      context.drawImage(bitmap, 0, 0, canvas.width, canvas.height);
      const thumbnail = await new Promise<Blob | null>((resolve) => canvas.toBlob(resolve, 'image/png'));
      if (!thumbnail) return original;
      URL.revokeObjectURL(original);
      if (!active()) return null;
      remember(key, thumbnail);
      return URL.createObjectURL(thumbnail);
    } finally { bitmap.close(); }
  } catch {
    if (!active()) { URL.revokeObjectURL(original); return null; }
    return original;
  }
}
