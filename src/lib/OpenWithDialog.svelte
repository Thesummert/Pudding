<script lang="ts">
  import { X, ExternalLink, LoaderCircle } from '@lucide/svelte';
  import { api } from './api';
  import type { FileEntry } from './types';

  let { entry, onclose }: { entry: FileEntry; onclose: () => void } = $props();
  let dialog: HTMLDialogElement;
  let apps = $state<{ id: string; name: string }[]>([]);
  let loading = $state(true);
  let busy = $state(false);
  let error = $state('');
  let active = true;

  $effect(() => {
    dialog.showModal();
    api.listOpenApps(entry.path).then((result) => { if (active) apps = result; })
      .catch((reason) => { if (active) error = String(reason); })
      .finally(() => { if (active) loading = false; });
    return () => { active = false; };
  });

  async function open(application: string | null) {
    if (busy) return;
    busy = true;
    error = '';
    try { await api.openWith(entry.path, application); onclose(); }
    catch (reason) { error = String(reason); }
    finally { busy = false; }
  }
</script>

<dialog bind:this={dialog} class="modal open-with-modal" aria-labelledby="open-with-title" oncancel={(event) => { event.preventDefault(); if (!busy) onclose(); }}>
  <div class="modal-top"><span class="modal-icon"><ExternalLink size={23} /></span><button class="icon-button" title="关闭" aria-label="关闭" onclick={onclose} disabled={busy}><X size={18} /></button></div>
  <h2 id="open-with-title">选择打开方式</h2>
  <p>选择用于打开“{entry.name}”的应用。</p>
  <div class="open-with-list" aria-label="推荐应用">
    {#if loading}<p><LoaderCircle class="spin" size={16} /> 正在查找推荐应用…</p>
    {:else if !apps.length && !error}<p>系统没有返回推荐应用，可以手动选择。</p>
    {:else}{#each apps as app (app.id)}<button class="open-with-app" disabled={busy} onclick={() => open(app.id)}><ExternalLink size={16} /><span>{app.name}</span></button>{/each}{/if}
  </div>
  {#if error}<p class="field-error" role="alert">{error}</p>{/if}
  <div class="modal-actions"><button class="button" onclick={onclose} disabled={busy}>取消</button><button class="button primary" onclick={() => open(null)} disabled={busy}>选择其他应用…</button></div>
</dialog>
