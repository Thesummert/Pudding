<script lang="ts">
  import { FileText, FileCode2, FileArchive, File, Image, Music2, Film, Link2, Download, Monitor, Headphones, Camera, Code2 } from '@lucide/svelte';
  import { fileKind } from './files';
  import type { FileEntry } from './types';

  let { entry, large = false }: { entry: FileEntry; large?: boolean } = $props();
  const folderSymbols: Record<string, typeof Download> = { Downloads: Download, Desktop: Monitor, Documents: FileText, Pictures: Camera, Music: Headphones, Videos: Film, Projects: Code2 };
  let kind = $derived(fileKind(entry));
  let FolderSymbol = $derived(folderSymbols[entry.name]);
  let Icon = $derived(kind === '图像' ? Image : kind === '音频' ? Music2 : kind === '视频' ? Film : kind === '压缩文件' ? FileArchive : kind === '代码文件' ? FileCode2 : kind.includes('文档') ? FileText : File);
</script>

<span class:large class:folder={entry.isDir} class="file-icon" data-kind={kind} aria-hidden="true">
  {#if entry.isDir}
    <svg class="folder-shape" viewBox="0 0 72 60" fill="none">
      <path d="M5 13a5 5 0 0 1 5-5h17l7 7h28a5 5 0 0 1 5 5v28a5 5 0 0 1-5 5H10a5 5 0 0 1-5-5V13Z" fill="var(--folder-back)" />
      <path d="M5 23a5 5 0 0 1 5-5h52a5 5 0 0 1 5 5v25a5 5 0 0 1-5 5H10a5 5 0 0 1-5-5V23Z" fill="var(--folder-front)" />
      <path d="M10 18h52a5 5 0 0 1 5 5v1H5v-1a5 5 0 0 1 5-5Z" fill="white" fill-opacity=".14" />
    </svg>
    {#if FolderSymbol}<FolderSymbol class="folder-symbol" size={large ? 23 : 13} strokeWidth={1.7} />{/if}
  {:else}
    <span class="document-shape"><Icon size={large ? 29 : 18} strokeWidth={1.5} /></span>
  {/if}
  {#if entry.isSymlink}<span class="link-badge"><Link2 size={11} /></span>{/if}
</span>
