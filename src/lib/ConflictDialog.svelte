<script lang="ts">
  import { CircleAlert, X } from '@lucide/svelte';
  import { basename } from './files';
  let { conflicts, onchoose }: { conflicts: string[]; onchoose: (choice: 'replace' | 'keep' | 'cancel') => void } = $props();
  let dialog: HTMLDialogElement;
  $effect(() => { dialog.showModal(); });
</script>

<dialog bind:this={dialog} class="modal conflict-modal" aria-labelledby="conflict-title" oncancel={(event) => { event.preventDefault(); onchoose('cancel'); }}>
  <div class="modal-top"><span class="modal-icon"><CircleAlert size={24} /></span><button class="icon-button" aria-label="关闭" onclick={() => onchoose('cancel')}><X size={18} /></button></div>
  <h2 id="conflict-title">目标位置已有同名项目</h2>
  <p>{conflicts.length === 1 ? `「${basename(conflicts[0])}」已存在。` : `有 ${conflicts.length} 个项目名称冲突。`}替换会用所选项目完整替换目标，包括文件夹内的内容；并存会为新项目添加副本名称。</p>
  <div class="modal-actions"><button class="button" onclick={() => onchoose('cancel')}>取消</button><button class="button" onclick={() => onchoose('keep')}>并存</button><button class="button primary" onclick={() => onchoose('replace')}>替换</button></div>
</dialog>
