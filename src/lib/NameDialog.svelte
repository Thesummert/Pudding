<script lang="ts">
  import { tick, untrack } from 'svelte';
  import { FolderPlus, Pencil, X, LoaderCircle } from '@lucide/svelte';
  import { validateName } from './files';

  let { mode, initial = '', busy, error, onsubmit, onclose }: { mode: 'create' | 'rename'; initial?: string; busy: boolean; error: string; onsubmit: (name: string) => void; onclose: () => void } = $props();
  let value = $state(untrack(() => initial));
  let input: HTMLInputElement;
  let invalid = $derived(value.length ? validateName(value) : null);
  let dialog: HTMLDialogElement;

  $effect(() => {
    dialog.showModal();
    void tick().then(() => {
      input.focus();
      const dot = initial.lastIndexOf('.');
      input.setSelectionRange(0, mode === 'rename' && dot > 0 ? dot : initial.length);
    });
  });
</script>

<dialog bind:this={dialog} class="modal" oncancel={(event) => { event.preventDefault(); if (!busy) onclose(); }} onclose={onclose}>
  <form onsubmit={(event) => { event.preventDefault(); if (!invalid && value.trim() && !busy) onsubmit(value); }}>
    <div class="modal-top"><span class="modal-icon">{#if mode === 'create'}<FolderPlus size={23} />{:else}<Pencil size={23} />{/if}</span><button class="icon-button" type="button" title="关闭" onclick={onclose} disabled={busy}><X size={18} /></button></div>
    <h2>{mode === 'create' ? '新建文件夹' : '重命名'}</h2>
    <p>{mode === 'create' ? '为新的空间起一个名字。' : '输入此项目的新名称。'}</p>
    <label class="field-label" for="entry-name">名称</label>
    <input id="entry-name" class="name-input" bind:this={input} bind:value autocomplete="off" spellcheck="false" disabled={busy} />
    <div class="field-error" aria-live="polite">{error || invalid || ''}</div>
    <div class="modal-actions"><button type="button" class="button" onclick={onclose} disabled={busy}>取消</button><button class="button primary" type="submit" disabled={busy || !!invalid || !value.trim()}>{#if busy}<LoaderCircle class="spin" size={16} />{/if}{mode === 'create' ? '创建文件夹' : '保存名称'}</button></div>
  </form>
</dialog>
