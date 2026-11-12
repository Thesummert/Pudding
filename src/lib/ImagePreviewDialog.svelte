<script lang="ts">
  import { onMount, untrack } from 'svelte';
  import { ArrowLeft, ArrowRight, ExternalLink, Maximize, Scan, X } from '@lucide/svelte';
  import ImagePreview from './ImagePreview.svelte';
  import { formatSize } from './files';
  import type { FileEntry } from './types';

  let { entries, initial, onclose, onopen }: { entries: FileEntry[]; initial: FileEntry; onclose: () => void; onopen: (entry: FileEntry) => void } = $props();
  let path = $state(untrack(() => initial.path));
  let actualSize = $state(false);
  let dialog: HTMLDialogElement;
  let index = $derived(entries.findIndex((entry) => entry.path === path));
  let current = $derived(entries[index] ?? initial);

  onMount(() => dialog.showModal());

  function close() {
    dialog.close();
    // Notify now; a queued native close event could dismiss a newly opened preview.
    onclose();
  }

  function step(offset: number) {
    const next = entries[index + offset];
    if (!next) return;
    path = next.path;
    actualSize = false;
  }

  function keydown(event: KeyboardEvent) {
    if (event.altKey || event.ctrlKey || event.metaKey) return;
    if (event.key === 'ArrowLeft' || event.key === 'ArrowRight') {
      event.preventDefault();
      event.stopPropagation();
      step(event.key === 'ArrowLeft' ? -1 : 1);
    }
  }
</script>

<dialog bind:this={dialog} class="modal image-modal" aria-labelledby="image-preview-title" oncancel={(event) => { event.preventDefault(); close(); }} onkeydown={keydown}>
  <div class="image-modal-header">
    <div class="image-modal-title"><h2 id="image-preview-title">{current.name}</h2><span>{formatSize(current.size)} · {index + 1} / {entries.length}</span></div>
    <button class="button" onclick={() => actualSize = !actualSize} aria-pressed={actualSize}>{#if actualSize}<Scan size={16} />适应窗口{:else}<Maximize size={16} />实际大小{/if}</button>
    <button class="button" onclick={() => { close(); onopen(current); }}><ExternalLink size={16} />使用默认应用打开</button>
    <button class="icon-button" title="关闭图片预览 (Esc)" aria-label="关闭图片预览" onclick={close}><X size={20} /></button>
  </div>
  <div class="image-modal-stage">
    <button class="icon-button image-step" title="上一张 (←)" aria-label="上一张图片" disabled={index <= 0} onclick={() => step(-1)}><ArrowLeft size={22} /></button>
    {#key current.path}<ImagePreview entry={current} {actualSize} />{/key}
    <button class="icon-button image-step" title="下一张 (→)" aria-label="下一张图片" disabled={index < 0 || index >= entries.length - 1} onclick={() => step(1)}><ArrowRight size={22} /></button>
  </div>
  <div class="image-modal-footer">← / → 切换图片<span>Esc 关闭预览</span></div>
</dialog>
