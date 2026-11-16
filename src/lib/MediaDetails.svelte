<script lang="ts">
  import { api } from './api';
  import { isImage, isVideo } from './files';
  import type { FileEntry } from './types';

  let { entry }: { entry: FileEntry } = $props();
  let details = $state<{ width: number; height: number; duration?: number } | null>(null);
  let failed = $state(false);

  $effect(() => {
    const file = entry;
    let active = true;
    let source: string | undefined;
    let media: HTMLVideoElement | undefined;
    let image: HTMLImageElement | undefined;
    details = null;
    failed = false;
    const load = async () => {
      try {
        if (isImage(file)) {
          const url = await api.image(file.path, file.extension);
          if (!active) { URL.revokeObjectURL(url); return; }
          source = url;
          image = new Image();
          image.onload = () => { if (active) details = { width: image!.naturalWidth, height: image!.naturalHeight }; };
          image.onerror = () => { if (active) failed = true; };
          image.src = url;
        } else if (isVideo(file)) {
          const url = await api.videoSource(file.path);
          if (!active) return;
          media = document.createElement('video');
          media.preload = 'metadata';
          media.onloadedmetadata = () => {
            if (active) details = { width: media!.videoWidth, height: media!.videoHeight, duration: media!.duration };
          };
          media.onerror = () => { if (active) failed = true; };
          media.src = url;
        }
      } catch { if (active) failed = true; }
    };
    void load();
    return () => {
      active = false;
      if (image) { image.onload = null; image.onerror = null; image.src = ''; }
      if (media) { media.onloadedmetadata = null; media.onerror = null; media.removeAttribute('src'); media.load(); }
      if (source) URL.revokeObjectURL(source);
    };
  });

  function durationText(seconds: number): string {
    const total = Math.floor(seconds);
    const hours = Math.floor(total / 3600);
    const minutes = Math.floor(total % 3600 / 60);
    const remainder = total % 60;
    return hours ? `${hours}:${String(minutes).padStart(2, '0')}:${String(remainder).padStart(2, '0')}` : `${minutes}:${String(remainder).padStart(2, '0')}`;
  }
  function bitrateText(bits: number): string {
    return bits >= 1_000_000 ? `${(bits / 1_000_000).toFixed(2)} Mbps` : `${Math.round(bits / 1000)} kbps`;
  }
</script>

{#if isImage(entry) || isVideo(entry)}
  {#if failed}
    <div><dt>媒体信息</dt><dd>无法读取</dd></div>
  {:else if !details}
    <div><dt>媒体信息</dt><dd>正在读取…</dd></div>
  {:else}
    <div><dt>分辨率</dt><dd>{details.width && details.height ? `${details.width} × ${details.height} 像素` : '未知'}</dd></div>
    {#if isVideo(entry)}
      <div><dt>时长</dt><dd>{details.duration !== undefined && Number.isFinite(details.duration) && details.duration > 0 ? durationText(details.duration) : '未知'}</dd></div>
      <div><dt>平均总码率</dt><dd>{details.duration !== undefined && Number.isFinite(details.duration) && details.duration > 0 && !entry.metadataPending && !entry.metadataUnavailable && entry.size > 0 ? `约 ${bitrateText(entry.size * 8 / details.duration)}` : '未知'}</dd></div>
    {/if}
  {/if}
{/if}
