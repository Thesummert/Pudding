<script lang="ts">
  import { CircleAlert, Link2, LoaderCircle } from '@lucide/svelte';
  import FileIcon from './FileIcon.svelte';
  import { api } from './api';
  import type { FileEntry } from './types';

  let { entry, actualSize = false, thumbnail = false, large = false }: { entry: FileEntry; actualSize?: boolean; thumbnail?: boolean; large?: boolean } = $props();
  let source = $state('');
  let loaded = $state(false);
  let error = $state('');

  $effect(() => {
    const { path, extension, modified, size } = entry;
    // Include metadata so refreshing an externally modified image reloads it.
    void modified;
    void size;
    let active = true;
    let url = '';
    source = '';
    loaded = false;
    error = '';
    void api.image(path, extension).then((result) => {
      if (!active) { URL.revokeObjectURL(result); return; }
      url = result;
      source = result;
    }).catch((reason: unknown) => {
      if (active) error = reason instanceof Error ? reason.message : String(reason);
    });
    return () => { active = false; if (url) URL.revokeObjectURL(url); };
  });
</script>

<div class="image-preview" class:thumbnail class:actual-size={actualSize} aria-busy={!loaded && !error} title={thumbnail && error ? error : undefined}>
  {#if thumbnail}
    {#if source && !error}<img src={source} alt={entry.name} class:pending={!loaded} decoding="async" onload={() => loaded = true} onerror={() => error = '图片可能已损坏，或当前系统不支持此格式。可使用默认应用打开。'} />{/if}
    {#if !loaded || error}<FileIcon {entry} {large} />{:else if entry.isSymlink}<span class="link-badge" aria-hidden="true"><Link2 size={11} /></span>{/if}
  {:else if error}
    <div class="image-preview-message" role="status"><CircleAlert size={24} /><span>无法预览图片</span><small>{error}</small></div>
  {:else}
    {#if source}<img src={source} alt={entry.name} class:pending={!loaded} onload={() => loaded = true} onerror={() => error = '图片可能已损坏，或当前系统不支持此格式。可使用默认应用打开。'} />{/if}
    {#if !loaded}<div class="image-preview-message" role="status"><LoaderCircle size={22} class="spin" /><span>正在加载图片…</span></div>{/if}
  {/if}
</div>
