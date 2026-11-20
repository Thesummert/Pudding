<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from './api';
  import FileIcon from './FileIcon.svelte';
  import FileThumbnail from './FileThumbnail.svelte';
  import { isImage } from './files';
  import { queueThumbnail } from './thumbnail-queue';
  import type { FileEntry } from './types';

  let { entry, hidden, version }: { entry: FileEntry; hidden: boolean; version: number } = $props();
  let folderPath = $derived(entry.path);
  let container: HTMLSpanElement;
  let visible = $state(false);
  let children = $state<FileEntry[]>([]);
  let shown = $derived(children.filter((child) => hidden || !child.hidden));

  onMount(() => {
    if (typeof IntersectionObserver === 'undefined') { visible = true; return; }
    const observer = new IntersectionObserver(([item]) => visible = item.isIntersecting, {
      root: container.closest('.file-area'), rootMargin: '120px',
    });
    observer.observe(container);
    return () => observer.disconnect();
  });

  $effect(() => {
    const path = folderPath;
    const showHidden = hidden;
    void version;
    if (!visible) return;
    let active = true;
    void queueThumbnail(() => api.folderPreview(path, showHidden), () => active)
      .then((items) => { if (active && items) children = items; })
      .catch(() => { if (active) children = []; });
    return () => { active = false; };
  });
</script>

<span class="folder-preview" bind:this={container} aria-hidden="true">
  <FileIcon {entry} large />
  {#if shown.length}
    <span class="folder-preview-items">
      {#each shown as child (child.path)}
        <span class="folder-preview-item">
          {#if isImage(child)}<FileThumbnail entry={child} />{:else}<FileIcon entry={child} />{/if}
        </span>
      {/each}
    </span>
  {/if}
</span>
