<script lang="ts">
  import { onMount } from 'svelte';
  import FileIcon from './FileIcon.svelte';
  import ImagePreview from './ImagePreview.svelte';
  import VideoThumbnail from './VideoThumbnail.svelte';
  import { isVideo } from './files';
  import type { FileEntry } from './types';

  let { entry, large = false }: { entry: FileEntry; large?: boolean } = $props();
  let container: HTMLSpanElement;
  let visible = $state(false);

  onMount(() => {
    if (typeof IntersectionObserver === 'undefined') {
      visible = true;
      return;
    }
    const observer = new IntersectionObserver(([item]) => visible = item.isIntersecting, {
      root: container.closest('.file-area'),
      rootMargin: '120px',
    });
    observer.observe(container);
    return () => observer.disconnect();
  });
</script>

<span bind:this={container} class="file-thumbnail" class:large>
  {#if visible}{#if isVideo(entry)}<VideoThumbnail {entry} {large} />{:else}<ImagePreview {entry} thumbnail {large} />{/if}{:else}<FileIcon {entry} {large} />{/if}
</span>
