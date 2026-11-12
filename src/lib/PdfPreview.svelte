<script lang="ts">
  import { ChevronLeft, ChevronRight, LoaderCircle } from '@lucide/svelte';
  import { api } from './api';
  import workerUrl from 'pdfjs-dist/build/pdf.worker.min.mjs?url';
  import type { PDFDocumentProxy, PDFDocumentLoadingTask, RenderTask } from 'pdfjs-dist';
  import type { FileEntry } from './types';

  const cmaps = import.meta.glob('/node_modules/pdfjs-dist/cmaps/*.bcmap', { eager: true, query: '?url&no-inline', import: 'default' }) as Record<string, string>;
  class BundledCMapReader {
    async fetch({ name }: { name: string }) {
      const url = cmaps[`/node_modules/pdfjs-dist/cmaps/${name}.bcmap`];
      if (!url) throw new Error('PDF 字符映射不可用');
      const data = await new Promise<ArrayBuffer>((resolve, reject) => {
        const request = new XMLHttpRequest();
        request.open('GET', url);
        request.responseType = 'arraybuffer';
        request.onload = () => request.status === 200 || request.status === 0 ? resolve(request.response) : reject(new Error('无法读取 PDF 字符映射'));
        request.onerror = () => reject(new Error('无法读取 PDF 字符映射'));
        request.send();
      });
      return { cMapData: new Uint8Array(data), isCompressed: true };
    }
  }

  let { entry }: { entry: FileEntry } = $props();
  let pdf = $state<PDFDocumentProxy | null>(null);
  let page = $state(1);
  let error = $state('');
  let rendering = $state(false);
  let canvas: HTMLCanvasElement;
  let container: HTMLDivElement;

  $effect(() => {
    const path = entry.path;
    let active = true;
    let task: PDFDocumentLoadingTask | undefined;
    pdf = null;
    page = 1;
    error = '';
    void (async () => {
      const [pdfjs, bytes] = await Promise.all([import('pdfjs-dist'), api.previewBytes(path, 'pdf')]);
      if (!active) return;
      pdfjs.GlobalWorkerOptions.workerSrc = workerUrl;
      task = pdfjs.getDocument({ data: bytes, isEvalSupported: false, CMapReaderFactory: BundledCMapReader, useWorkerFetch: false });
      const result = await task.promise;
      if (active) pdf = result;
    })().catch((reason: unknown) => { if (active) error = reason instanceof Error ? reason.message : String(reason); });
    return () => { active = false; void task?.destroy(); };
  });

  $effect(() => {
    const doc = pdf, number = page;
    if (!doc || !canvas || !container) return;
    let active = true;
    let task: RenderTask | undefined;
    rendering = true;
    void (async () => {
      const current = await doc.getPage(number);
      if (!active) return;
      const initial = current.getViewport({ scale: 1 });
      if (!Number.isFinite(initial.width) || !Number.isFinite(initial.height) || initial.width <= 0 || initial.height <= 0) throw new Error('PDF 页面尺寸无效');
      // Bound canvas allocation for unusually large or malformed pages.
      const scale = Math.min(2, Math.max(0.1, (container.clientWidth - 40) / initial.width), 8192 / initial.width, 8192 / initial.height, Math.sqrt(16_000_000 / (initial.width * initial.height)));
      const viewport = current.getViewport({ scale });
      const context = canvas.getContext('2d');
      if (!context) throw new Error('无法显示 PDF 页面');
      canvas.width = Math.ceil(viewport.width);
      canvas.height = Math.ceil(viewport.height);
      task = current.render({ canvasContext: context, viewport });
      await task.promise;
      if (active) rendering = false;
    })().catch((reason: unknown) => { if (active) { rendering = false; error = reason instanceof Error ? reason.message : String(reason); } });
    return () => { active = false; task?.cancel(); };
  });
</script>

<div class="pdf-preview">
  {#if pdf && !error}
    <div class="pdf-controls"><button class="icon-button compact" aria-label="上一页" disabled={page <= 1} onclick={() => page--}><ChevronLeft size={18} /></button><span>{page} / {pdf.numPages}</span><button class="icon-button compact" aria-label="下一页" disabled={page >= pdf.numPages} onclick={() => page++}><ChevronRight size={18} /></button>{#if rendering}<LoaderCircle size={14} class="spin" />{/if}</div>
  {/if}
  <div class="pdf-page" bind:this={container}>
    <canvas bind:this={canvas} class:pending={!pdf || !!error} aria-label="PDF 页面"></canvas>
    {#if error}<div class="file-preview-message" role="status">无法预览 PDF：{error}</div>{:else if !pdf}<div class="file-preview-message" role="status"><LoaderCircle size={22} class="spin" />正在加载 PDF…</div>{/if}
  </div>
</div>
