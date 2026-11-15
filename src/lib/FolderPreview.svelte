<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from './api';
  import FileIcon from './FileIcon.svelte';
  import FileThumbnail from './FileThumbnail.svelte';
  import { isImage } from './files';
  import { queueThumbnail } from './thumbnail-queue';
  import type { DirectoryListing, FileEntry } from './types';

  let { entry, hidden, refreshToken }: { entry: FileEntry; hidden: boolean; refreshToken: DirectoryListing | undefined } = $props();
  let container: HTMLSpanElement;
  let visible = $state(false);
  let children = $state<FileEntry[]>([]);

  onMount(() => {
    if (typeof IntersectionObserver === 'undefined') { visible = true; return; }
    const observer = new IntersectionObserver(([item]) => visible = item.isIntersecting, {
      root: container.closest('.file-area'), rootMargin: '120px',
    });
    observer.observe(container);
    return () => observer.disconnect();
  });

  $effect(() => {
    const path = entry.path;
    const showHidden = hidden;
    // A new listing also invalidates previews after filesystem changes or refresh.
    void refreshToken;
    children = [];
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
  {#if children.length}
    <span class="folder-preview-items">
      {#each children as child (child.path)}
        <span class="folder-preview-item">
          {#if isImage(child)}<FileThumbnail entry={child} />{:else}<FileIcon entry={child} />{/if}
        </span>
      {/each}
    </span>
  {/if}
</span>
