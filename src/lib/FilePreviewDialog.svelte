<script lang="ts">
  import { onMount } from 'svelte';
  import { ExternalLink, LoaderCircle, X } from '@lucide/svelte';
  import PdfPreview from './PdfPreview.svelte';
  import { api } from './api';
  import { formatSize, isPdf, isText, isVideo } from './files';
  import type { FileEntry } from './types';

  let { entry, onclose, onopen }: { entry: FileEntry; onclose: () => void; onopen: (entry: FileEntry) => void } = $props();
  let dialog: HTMLDialogElement;
  let content = $state('');
  let source = $state('');
  let error = $state('');
  let loading = $state(true);
  let backdropDown = false;

  onMount(() => dialog.showModal());

  $effect(() => {
    const path = entry.path;
    let active = true;
    content = ''; source = ''; error = ''; loading = true;
    if (isText(entry)) {
      void api.previewBytes(path, 'text').then((bytes) => {
        if (!active) return;
        let encoding = 'utf-8';
        if (bytes[0] === 0xff && bytes[1] === 0xfe) encoding = 'utf-16le';
        else if (bytes[0] === 0xfe && bytes[1] === 0xff) encoding = 'utf-16be';
        const text = new TextDecoder(encoding, { fatal: true }).decode(bytes);
        if (text.includes('\0')) throw new Error('文件包含二进制数据，无法作为文本预览。');
        content = text; loading = false;
      }).catch((reason: unknown) => { if (active) { error = reason instanceof Error ? reason.message : String(reason); loading = false; } });
    } else if (isVideo(entry)) {
      void api.videoSource(path).then((url) => { if (active) { source = url; loading = false; } }).catch((reason: unknown) => { if (active) { error = reason instanceof Error ? reason.message : String(reason); loading = false; } });
    } else loading = false;
    return () => { active = false; };
  });

  async function startPlayback(event: Event) {
    const video = event.currentTarget as HTMLVideoElement;
    try {
      await video.play();
    } catch (reason) {
      // Only retry when autoplay policy blocks sound, not when a user pauses.
      if (!(reason instanceof Error) || reason.name !== 'NotAllowedError' || !video.isConnected || !video.paused) return;
      video.muted = true;
      try { await video.play(); } catch { /* Playback remains available through the controls. */ }
    }
  }

  function close() { dialog.close(); onclose(); }
  function outside(event: MouseEvent) {
    const rect = dialog.getBoundingClientRect();
    return event.target === dialog && (event.clientX < rect.left || event.clientX > rect.right || event.clientY < rect.top || event.clientY > rect.bottom);
  }
  function keydown(event: KeyboardEvent) {
    if (event.code === 'Space' && !event.altKey && !event.ctrlKey && !event.metaKey && !event.repeat) {
      event.preventDefault(); event.stopPropagation(); close();
    }
  }
</script>

<dialog bind:this={dialog} class="modal file-preview-modal" aria-labelledby="file-preview-title" onpointerdown={(event) => backdropDown = event.button === 0 && outside(event)} onclick={(event) => { if (backdropDown && outside(event)) close(); backdropDown = false; }} oncancel={(event) => { event.preventDefault(); close(); }} onkeydown={keydown}>
  <div class="image-modal-header"><div class="image-modal-title"><h2 id="file-preview-title">{entry.name}</h2><span>{formatSize(entry.size)}</span></div><button class="button" onclick={() => { close(); onopen(entry); }}><ExternalLink size={16} />使用默认应用打开</button><button class="icon-button" title="关闭预览 (Esc)" aria-label="关闭预览" onclick={close}><X size={20} /></button></div>
  <div class="file-preview-stage">
    {#if error}<div class="file-preview-message" role="status">无法预览：{error}</div>
    {:else if loading}<div class="file-preview-message" role="status"><LoaderCircle size={22} class="spin" />正在加载…</div>
    {:else if isText(entry)}<div class="text-preview" role="textbox" aria-readonly="true" aria-label="文本内容" tabindex="0"><pre>{content}</pre></div>
    {:else if isVideo(entry)}<video class="video-preview" controls autoplay playsinline preload="metadata" src={source} onloadedmetadata={startPlayback} onerror={() => error = '视频可能已损坏，或当前系统不支持此格式。'}><track kind="captions" /></video>
    {:else if isPdf(entry)}<PdfPreview {entry} />{/if}
  </div>
  <div class="image-modal-footer"><span>空格或 Esc 关闭预览</span></div>
</dialog>
