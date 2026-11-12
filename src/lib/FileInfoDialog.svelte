<script lang="ts">
  import { onMount } from 'svelte';
  import { X } from '@lucide/svelte';
  import FileIcon from './FileIcon.svelte';
  import { api } from './api';
  import { fileKind, formatDate, formatSize } from './files';
  import type { FileEntry } from './types';

  let { entry, onclose }: { entry: FileEntry; onclose: () => void } = $props();
  let dialog: HTMLDialogElement;
  let folderSize = $state<number | null>(null);
  let sizeError = $state(false);

  onMount(() => {
    dialog.showModal();
    let active = true;
    if (entry.isDir) void api.directorySize(entry.path).then((size) => { if (active) folderSize = size; }).catch(() => { if (active) sizeError = true; });
    return () => { active = false; };
  });

  function close() { dialog.close(); onclose(); }
  function keydown(event: KeyboardEvent) {
    if (event.code === 'Space' && !event.altKey && !event.ctrlKey && !event.metaKey && !event.repeat) {
      event.preventDefault(); event.stopPropagation(); close();
    }
  }
</script>

<dialog bind:this={dialog} class="modal info-preview-modal" aria-labelledby="info-preview-title" oncancel={(event) => { event.preventDefault(); close(); }} onkeydown={keydown}>
  <div class="modal-top"><span class="modal-icon"><FileIcon {entry} /></span><button class="icon-button" title="关闭信息 (Esc)" aria-label="关闭信息" onclick={close}><X size={18} /></button></div>
  <h2 id="info-preview-title">{entry.name}</h2>
  <p>基本信息</p>
  <dl class="info-preview-properties">
    <div><dt>类型</dt><dd>{fileKind(entry)}</dd></div>
    <div><dt>大小</dt><dd>{entry.isDir ? (sizeError ? '无法计算' : folderSize === null ? '正在计算…' : formatSize(folderSize)) : formatSize(entry.size)}</dd></div>
    <div><dt>修改时间</dt><dd>{formatDate(entry.modified)}</dd></div>
    <div><dt>位置</dt><dd>{entry.path}</dd></div>
  </dl>
  {#if entry.isDir}<p class="info-preview-note">文件夹大小为文件内容总计，不包含符号链接。</p>{/if}
  <div class="modal-actions"><button class="button" onclick={close}>关闭</button></div>
</dialog>
