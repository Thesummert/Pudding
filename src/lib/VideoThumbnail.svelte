<script lang="ts">
  import { Link2 } from '@lucide/svelte';
  import FileIcon from './FileIcon.svelte';
  import { api } from './api';
  import type { FileEntry } from './types';

  let { entry, large = false }: { entry: FileEntry; large?: boolean } = $props();
  let source = $state('');
  let loaded = $state(false);
  let failed = $state(false);

  function seekCover(event: Event) {
    const video = event.currentTarget as HTMLVideoElement;
    if (!Number.isFinite(video.duration) || video.duration <= 0) return;
    try { video.currentTime = Math.min(1, video.duration / 2); } catch { /* Use the first decoded frame if seeking is unavailable. */ }
  }

  $effect(() => {
    const { path, modified, size } = entry;
    let active = true;
    source = '';
    loaded = false;
    failed = false;
    void api.videoSource(path).then((url) => {
      if (active) source = url.startsWith('data:') ? url : `${url}${url.includes('?') ? '&' : '?'}v=${encodeURIComponent(`${modified ?? ''}-${size}`)}`;
    }).catch(() => { if (active) failed = true; });
    return () => { active = false; };
  });
</script>

<div class="image-preview thumbnail video-thumbnail" aria-hidden="true">
  {#if source && !failed}
    <video src={source} preload="auto" muted playsinline class:pending={!loaded} onloadedmetadata={seekCover} onloadeddata={() => loaded = true} onerror={() => failed = true}></video>
  {/if}
  {#if !loaded || failed}<FileIcon {entry} {large} />{:else if entry.isSymlink}<span class="link-badge"><Link2 size={11} /></span>{/if}
</div>
