<script lang="ts">
  import { CircleAlert, Link2, LoaderCircle } from '@lucide/svelte';
  import FileIcon from './FileIcon.svelte';
  import { api } from './api';
  import type { FileEntry } from './types';

  let { entry, actualSize = false, thumbnail = false, large = false, interactive = false }: { entry: FileEntry; actualSize?: boolean; thumbnail?: boolean; large?: boolean; interactive?: boolean } = $props();
  let source = $state('');
  let loaded = $state(false);
  let error = $state('');
  let zoom = $state(1);
  let offset = $state({ x: 0, y: 0 });
  let dragging = $state(false);

  $effect(() => {
    void actualSize;
    void entry.path;
    zoom = 1;
    offset = { x: 0, y: 0 };
  });

  function gestures(node: HTMLDivElement) {
    if (!interactive) return;
    let pointer: number | undefined;
    let previous = { x: 0, y: 0 };
    function wheel(event: WheelEvent) {
      event.preventDefault();
      if (!loaded || error) return;
      const delta = event.deltaY * (event.deltaMode === 1 ? 16 : event.deltaMode === 2 ? node.clientHeight : 1);
      const next = Math.max(0.1, Math.min(10, zoom * Math.exp(-delta * 0.002)));
      const factor = next / zoom;
      const rect = node.getBoundingClientRect();
      const x = event.clientX - rect.left - rect.width / 2;
      const y = event.clientY - rect.top - rect.height / 2;
      offset = { x: x - (x - offset.x) * factor, y: y - (y - offset.y) * factor };
      zoom = next;
    }
    function down(event: PointerEvent) {
      if (event.button !== 0 || pointer !== undefined || !loaded || error) return;
      event.preventDefault();
      node.setPointerCapture(event.pointerId);
      pointer = event.pointerId;
      previous = { x: event.clientX, y: event.clientY };
      dragging = true;
    }
    function move(event: PointerEvent) {
      if (event.pointerId !== pointer) return;
      offset = { x: offset.x + event.clientX - previous.x, y: offset.y + event.clientY - previous.y };
      previous = { x: event.clientX, y: event.clientY };
    }
    function up(event: PointerEvent) {
      if (event.pointerId !== pointer) return;
      pointer = undefined;
      dragging = false;
      if (node.hasPointerCapture(event.pointerId)) node.releasePointerCapture(event.pointerId);
    }
    const preventDrag = (event: DragEvent) => event.preventDefault();
    node.addEventListener('wheel', wheel, { passive: false });
    node.addEventListener('pointerdown', down);
    node.addEventListener('pointermove', move);
    node.addEventListener('pointerup', up);
    node.addEventListener('pointercancel', up);
    node.addEventListener('lostpointercapture', up);
    node.addEventListener('dragstart', preventDrag);
    return { destroy() {
      node.removeEventListener('wheel', wheel);
      node.removeEventListener('pointerdown', down);
      node.removeEventListener('pointermove', move);
      node.removeEventListener('pointerup', up);
      node.removeEventListener('pointercancel', up);
      node.removeEventListener('lostpointercapture', up);
      node.removeEventListener('dragstart', preventDrag);
    } };
  }

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

<div use:gestures class="image-preview" class:interactive class:dragging class:thumbnail class:actual-size={actualSize} aria-busy={!loaded && !error} title={thumbnail && error ? error : undefined}>
  {#if thumbnail}
    {#if source && !error}<img src={source} alt={entry.name} class:pending={!loaded} decoding="async" onload={() => loaded = true} onerror={() => error = '图片可能已损坏，或当前系统不支持此格式。可使用默认应用打开。'} />{/if}
    {#if !loaded || error}<FileIcon {entry} {large} />{:else if entry.isSymlink}<span class="link-badge" aria-hidden="true"><Link2 size={11} /></span>{/if}
  {:else if error}
    <div class="image-preview-message" role="status"><CircleAlert size={24} /><span>无法预览图片</span><small>{error}</small></div>
  {:else}
    {#if source}<img src={source} alt={entry.name} class:pending={!loaded} style:transform={interactive ? `translate(${offset.x}px, ${offset.y}px) scale(${zoom})` : undefined} onload={() => loaded = true} onerror={() => error = '图片可能已损坏，或当前系统不支持此格式。可使用默认应用打开。'} />{/if}
    {#if !loaded}<div class="image-preview-message" role="status"><LoaderCircle size={22} class="spin" /><span>正在加载图片…</span></div>{/if}
    {#if interactive && loaded}<span class="image-zoom" aria-label="缩放比例">{Math.round(zoom * 100)}%</span>{/if}
  {/if}
</div>
