<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { ArrowLeft, ArrowRight, ArrowUp, ArrowDown, ChevronRight, ChevronDown, Home, Monitor, FileText, Download, Image, Music2, Film, HardDrive, Folder, FolderPlus, Search, X, RotateCw, LayoutGrid, List, PanelRight, MoreHorizontal, Eye, EyeOff, Copy, Scissors, ClipboardPaste, Pencil, Trash2, ExternalLink, Check, Info, LoaderCircle, Keyboard, Sun, Moon, Link2, ChevronsUpDown, CircleAlert } from '@lucide/svelte';
  import FileIcon from './lib/FileIcon.svelte';
  import NameDialog from './lib/NameDialog.svelte';
  import { api, desktop } from './lib/api';
  import { basename, breadcrumbs, fileKind, formatDate, formatSize, visibleEntries } from './lib/files';
  import type { AppInfo, Clipboard, DirectoryListing, FileEntry, SortKey, ViewMode } from './lib/types';

  type Preferences = { view: ViewMode; hidden: boolean; inspector: boolean; size: number; dark: boolean };
  function loadPreferences(): Preferences {
    const defaults: Preferences = { view: 'grid', hidden: false, inspector: true, size: 120, dark: false };
    try {
      const saved = JSON.parse(localStorage.getItem('pudding.preferences') || '{}');
      return { view: saved.view === 'list' ? 'list' : 'grid', hidden: saved.hidden === true, inspector: saved.inspector !== false, size: Number.isFinite(saved.size) && saved.size >= 104 && saved.size <= 168 ? saved.size : 120, dark: saved.dark === true };
    } catch { return defaults; }
  }

  let preferences = $state(loadPreferences());
  let info = $state<AppInfo>();
  let directory = $state<DirectoryListing>();
  let loading = $state(true);
  let operating = $state(false);
  let initialError = $state('');
  let query = $state('');
  let sort = $state<SortKey>('name');
  let descending = $state(false);
  let selected = $state(new Set<string>());
  let anchor = $state<string>();
  let history = $state<string[]>([]);
  let historyIndex = $state(-1);
  let clipboard = $state<Clipboard>(null);
  let editingAddress = $state(false);
  let address = $state('');
  let addressInput = $state<HTMLInputElement>();
  let searchInput: HTMLInputElement;
  let fileArea: HTMLDivElement;
  let request = 0;
  let notice = $state<{ text: string; error: boolean } | null>(null);
  let noticeTimeout: ReturnType<typeof setTimeout>;
  let dialog = $state<{ mode: 'create' | 'rename'; entry?: FileEntry } | null>(null);
  let dialogError = $state('');
  let trashDialog: HTMLDialogElement;
  let helpDialog: HTMLDialogElement;
  let context = $state<{ x: number; y: number } | null>(null);
  let optionsOpen = $state(false);

  const placeIcons: Record<string, typeof Home> = { home: Home, desktop: Monitor, documents: FileText, downloads: Download, pictures: Image, music: Music2, videos: Film };
  const folderLabels: Record<string, string> = { Desktop: '桌面', Documents: '文档', Downloads: '下载', Music: '音乐', Pictures: '图片', Videos: '视频', Projects: '项目', Archive: '归档' };
  const shortcutModifier = /Mac|iPhone|iPad/.test(navigator.platform) ? '⌘' : 'Ctrl';

  let entries = $derived(visibleEntries(directory?.entries ?? [], query, preferences.hidden, sort, descending));
  let selectedEntries = $derived(entries.filter((entry) => selected.has(entry.path)));
  let focused = $derived(selectedEntries.length === 1 ? selectedEntries[0] : undefined);
  let currentPlace = $derived(info?.places.find((place) => place.path === directory?.path));
  let title = $derived(currentPlace?.name ?? (directory ? basename(directory.path) : '主文件夹'));
  let folderCount = $derived(entries.filter((e) => e.isDir).length);
  let fileCount = $derived(entries.length - folderCount);
  let crumbs = $derived(directory ? breadcrumbs(directory.path) : []);
  let busy = $derived(loading || operating);
  let selectedSize = $derived(selectedEntries.reduce((total, entry) => total + (entry.isDir ? 0 : entry.size), 0));
  let inspectorEntry = $derived(focused ?? { name: title, path: directory?.path ?? '', isDir: true, isSymlink: false, size: 0, modified: null, hidden: false, extension: '' });

  $effect(() => {
    try { localStorage.setItem('pudding.preferences', JSON.stringify(preferences)); } catch { /* Optional persistence. */ }
  });

  // Filtering must also update selection, so hidden files cannot be operated on accidentally.
  $effect(() => {
    const visiblePaths = new Set(entries.map((e) => e.path));
    if ([...selected].some((path) => !visiblePaths.has(path))) selected = new Set([...selected].filter((path) => visiblePaths.has(path)));
  });

  onMount(() => {
    void initialize();
    return () => { request += 1; clearTimeout(noticeTimeout); };
  });

  function notify(text: string, error = false) {
    clearTimeout(noticeTimeout);
    notice = { text, error };
    noticeTimeout = setTimeout(() => notice = null, error ? 8000 : 3500);
  }

  function errorMessage(error: unknown): string {
    return error instanceof Error ? error.message : String(error);
  }

  async function initialize() {
    loading = true;
    initialError = '';
    try {
      info = await api.info();
      await navigate(info.home);
    } catch (error) {
      initialError = errorMessage(error);
      loading = false;
    }
  }

  async function navigate(path: string, index?: number, refresh = false) {
    if (operating) return;
    const id = ++request;
    loading = true;
    context = null;
    try {
      const result = await api.read(path);
      if (id !== request) return;
      directory = result;
      editingAddress = false;
      initialError = '';
      if (!refresh) {
        query = '';
        selected = new Set();
        anchor = undefined;
      } else selected = new Set([...selected].filter((p) => result.entries.some((e) => e.path === p)));
      if (index !== undefined) historyIndex = index;
      else if (!refresh && history[historyIndex] !== result.path) {
        history = [...history.slice(0, historyIndex + 1), result.path];
        historyIndex = history.length - 1;
      }
      if (fileArea && !refresh) fileArea.scrollTop = 0;
    } catch (error) {
      if (id !== request) return;
      if (!directory) initialError = errorMessage(error);
      else notify(errorMessage(error), true);
    } finally { if (id === request) loading = false; }
  }

  function refresh() { if (directory && !busy) void navigate(directory.path, undefined, true); }
  function back() { if (!busy && historyIndex > 0) void navigate(history[historyIndex - 1], historyIndex - 1); }
  function forward() { if (!busy && historyIndex < history.length - 1) void navigate(history[historyIndex + 1], historyIndex + 1); }
  function up() { if (!busy && directory?.parent) void navigate(directory.parent); }

  async function editAddress() {
    address = directory?.path ?? '~';
    editingAddress = true;
    await tick();
    addressInput?.focus();
    addressInput?.select();
  }

  function selectEntry(entry: FileEntry, event: MouseEvent) {
    if (busy) return;
    context = null;
    if (event.shiftKey && anchor) {
      const start = entries.findIndex((e) => e.path === anchor);
      const end = entries.findIndex((e) => e.path === entry.path);
      if (start >= 0) selected = new Set(entries.slice(Math.min(start, end), Math.max(start, end) + 1).map((e) => e.path));
      else selected = new Set([entry.path]);
    } else if (event.metaKey || event.ctrlKey) {
      const next = new Set(selected);
      if (next.has(entry.path)) next.delete(entry.path); else next.add(entry.path);
      selected = next;
      anchor = entry.path;
    } else {
      selected = new Set([entry.path]);
      anchor = entry.path;
    }
  }

  async function openEntry(entry: FileEntry) {
    if (busy) return;
    if (entry.isDir) await navigate(entry.path);
    else {
      try { await api.open(entry.path); }
      catch (error) { notify(errorMessage(error), true); }
    }
  }

  function openDialog(mode: 'create' | 'rename') {
    if (busy || !directory || (mode === 'rename' && !focused)) return;
    context = null;
    optionsOpen = false;
    dialogError = '';
    dialog = { mode, entry: focused };
  }

  async function submitName(name: string) {
    if (!directory || !dialog || operating) return;
    operating = true;
    dialogError = '';
    try {
      const path = dialog.mode === 'create' ? await api.create(directory.path, name) : await api.rename(dialog.entry!.path, name);
      const message = dialog.mode === 'create' ? '文件夹已创建' : '名称已更新';
      dialog = null;
      operating = false;
      await navigate(directory.path, undefined, true);
      selected = new Set([path]);
      notify(message);
    } catch (error) { dialogError = errorMessage(error); }
    finally { operating = false; }
  }

  function copySelection(mode: 'copy' | 'cut') {
    if (!selected.size || busy) return;
    clipboard = { paths: [...selected], mode };
    context = null;
    notify(`已${mode === 'cut' ? '剪切' : '复制'} ${selected.size} 个项目，请到目标文件夹粘贴`);
  }

  async function paste() {
    if (!clipboard || !directory || busy) return;
    context = null;
    const operation = clipboard;
    operating = true;
    let paths: string[] = [];
    try {
      paths = await api.transfer([...operation.paths], directory.path, operation.mode);
      if (operation.mode === 'cut') clipboard = null;
      notify(`已${operation.mode === 'cut' ? '移动' : '复制'} ${paths.length} 个项目`);
    } catch (error) {
      // A batch can partially succeed; discard a cut operation rather than retry stale paths.
      if (operation.mode === 'cut') clipboard = null;
      notify(errorMessage(error), true);
    } finally {
      operating = false;
      await navigate(directory.path, undefined, true);
      if (paths.length) selected = new Set(paths);
    }
  }

  function confirmTrash() {
    if (!selected.size || busy) return;
    context = null;
    trashDialog.showModal();
  }

  async function moveToTrash() {
    if (!directory || busy || !selected.size) return;
    const paths = [...selected];
    operating = true;
    try {
      await api.trash(paths);
      clipboard = null;
      selected = new Set();
      notify(`已将 ${paths.length} 个项目移入回收站`);
    } catch (error) { notify(errorMessage(error), true); }
    finally {
      operating = false;
      trashDialog.close();
      await navigate(directory.path, undefined, true);
    }
  }

  async function copyPath() {
    context = null;
    const path = selectedEntries.length ? selectedEntries.map((e) => e.path).join('\n') : directory?.path;
    if (!path) return;
    try { await navigator.clipboard.writeText(path); notify('路径已复制到剪贴板'); }
    catch { notify('无法访问系统剪贴板，可在地址栏中手动复制路径。', true); }
  }

  function contextMenu(event: MouseEvent, entry?: FileEntry) {
    event.preventDefault();
    event.stopPropagation();
    if (busy) return;
    if (entry && !selected.has(entry.path)) selected = new Set([entry.path]);
    if (!entry) selected = new Set();
    optionsOpen = false;
    context = { x: Math.max(8, Math.min(event.clientX, innerWidth - 220)), y: Math.max(8, Math.min(event.clientY, innerHeight - 360)) };
  }

  function setSort(key: SortKey) {
    if (sort === key) descending = !descending;
    else { sort = key; descending = key === 'modified'; }
  }

  function keydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      context = null;
      optionsOpen = false;
      if (editingAddress) editingAddress = false;
      return;
    }
    if (document.querySelector('dialog[open]')) return;
    const target = event.target as HTMLElement;
    const modifier = event.ctrlKey || event.metaKey;
    if (modifier && event.key.toLowerCase() === 'l') { event.preventDefault(); void editAddress(); return; }
    if (modifier && event.key.toLowerCase() === 'f') { event.preventDefault(); searchInput.focus(); return; }
    if (target.closest('input, textarea, select, [contenteditable="true"]')) return;
    if (busy) return;
    const key = event.key.toLowerCase();
    if (event.altKey && key === 'arrowleft') { event.preventDefault(); back(); }
    else if (event.altKey && key === 'arrowright') { event.preventDefault(); forward(); }
    else if (event.altKey && key === 'arrowup') { event.preventDefault(); up(); }
    else if (key === 'backspace') { event.preventDefault(); up(); }
    else if (key === 'f5' || (modifier && key === 'r')) { event.preventDefault(); refresh(); }
    else if (modifier && event.shiftKey && key === 'n') { event.preventDefault(); openDialog('create'); }
    else if (modifier && event.shiftKey && event.code === 'Period') { event.preventDefault(); preferences.hidden = !preferences.hidden; }
    else if (modifier && key === 'a') { event.preventDefault(); selected = new Set(entries.map((e) => e.path)); }
    else if (modifier && key === 'c') { event.preventDefault(); copySelection('copy'); }
    else if (modifier && key === 'x') { event.preventDefault(); copySelection('cut'); }
    else if (modifier && key === 'v') { event.preventDefault(); void paste(); }
    else if (key === 'f2') { event.preventDefault(); openDialog('rename'); }
    else if (key === 'delete') { event.preventDefault(); confirmTrash(); }
    else if (key === 'enter' && focused && (target.closest('.file-area') || target === document.body)) { event.preventDefault(); void openEntry(focused); }
    else if (['arrowleft', 'arrowright', 'arrowup', 'arrowdown'].includes(key) && entries.length && (target.closest('.file-area') || target === document.body)) {
      event.preventDefault();
      const columns = preferences.view === 'list' ? 1 : Math.max(1, Math.floor((fileArea.clientWidth - 48 + 12) / (preferences.size + 12)));
      const offset = key === 'arrowleft' ? -1 : key === 'arrowright' ? 1 : key === 'arrowup' ? -columns : columns;
      const index = entries.findIndex((e) => e.path === anchor);
      const next = index < 0 ? 0 : Math.min(entries.length - 1, Math.max(0, index + offset));
      anchor = entries[next].path;
      selected = new Set([anchor]);
      void tick().then(() => fileArea.querySelector<HTMLButtonElement>(`[data-index="${next}"]`)?.focus());
    }
  }
</script>

<svelte:window onkeydown={keydown} onclick={() => { context = null; optionsOpen = false; }} onblur={() => { context = null; optionsOpen = false; }} onresize={() => context = null} />

<div class="app-shell" data-theme={preferences.dark ? 'dark' : 'light'}>
  <header class="app-header">
    <a class="brand" href="#home" onclick={(event) => { event.preventDefault(); if (info && !busy) void navigate(info.home); }} aria-label="Pudding 主文件夹">
      <span class="brand-mark"><svg viewBox="0 0 32 32" fill="none" aria-hidden="true"><path d="M6 11a2 2 0 0 1 2-2h7l3 3h7a2 2 0 0 1 2 2v10a2 2 0 0 1-2 2H8a2 2 0 0 1-2-2V11Z" fill="white" opacity=".95"/><path d="M6 16h21v8a2 2 0 0 1-2 2H8a2 2 0 0 1-2-2v-8Z" fill="#b9d2ff"/></svg></span>
      <span>Pudding<span class="brand-caption">文件管理器</span></span>
    </a>
    <div class="header-center"><span class="status-dot"></span>你的文件，井然有序。</div>
    <div class="header-actions">
      {#if !desktop}<span class="demo-badge" title="浏览器中使用示例数据；桌面版连接真实文件系统">演示模式</span>{:else}<span class="desktop-badge"><span class="status-dot"></span>本地工作空间</span>{/if}
      <button class="icon-button" title="快捷键与帮助" aria-label="快捷键与帮助" onclick={() => helpDialog.showModal()}><Keyboard size={18} /></button>
      <button class="icon-button" title={preferences.dark ? '切换浅色外观' : '切换深色外观'} aria-label={preferences.dark ? '切换浅色外观' : '切换深色外观'} onclick={() => preferences.dark = !preferences.dark}>{#if preferences.dark}<Sun size={18} />{:else}<Moon size={18} />{/if}</button>
    </div>
  </header>

  <div class="workspace">
    <aside class="sidebar" aria-label="位置侧栏">
      <div class="sidebar-scroll">
        <div class="section-label">常用位置<span>PLACES</span></div>
        <nav class="places" aria-label="常用位置">
          {#each info?.places ?? [] as place}
            {@const PlaceIcon = placeIcons[place.id] ?? Folder}
            <button class:active={directory?.path === place.path} onclick={() => navigate(place.path)} disabled={operating} aria-current={directory?.path === place.path ? 'page' : undefined} title={place.path}><PlaceIcon size={18} strokeWidth={1.8} /><span>{place.name}</span>{#if place.id === 'home'}<span class="home-key">⌂</span>{/if}</button>
          {/each}
          {#if !info}{#each Array(7) as _}<div class="sidebar-skeleton"></div>{/each}{/if}
        </nav>
        <div class="sidebar-divider"></div>
        <div class="section-label">设备与磁盘<span>DEVICES</span></div>
        <nav class="places devices" aria-label="设备与磁盘">
          {#each info?.roots ?? [] as root}
            <button class:active={directory?.path === root.path} onclick={() => navigate(root.path)} disabled={operating} title={root.path}><HardDrive size={18} strokeWidth={1.8} /><span>{root.name}</span></button>
          {/each}
        </nav>
        <div class="sidebar-note"><span class="note-icon"><Folder size={19} strokeWidth={1.5} /></span><p>给每一份灵感<br />一个妥帖的位置。</p><span class="note-line"></span></div>
      </div>
      <div class="sidebar-footer"><span class="small-brand">P</span><div><strong>Pudding</strong><span>一点轻巧，一切有序</span></div><span class="version">v0.1</span></div>
    </aside>

    <main class="main-shell">
      <div class="navigation-bar">
        <div class="navigation-controls">
          <button class="icon-button" title="后退 (Alt+←)" aria-label="后退" disabled={busy || historyIndex <= 0} onclick={back}><ArrowLeft size={19} /></button>
          <button class="icon-button" title="前进 (Alt+→)" aria-label="前进" disabled={busy || historyIndex >= history.length - 1} onclick={forward}><ArrowRight size={19} /></button>
          <button class="icon-button" title="上一级 (Alt+↑)" aria-label="上一级" disabled={busy || !directory?.parent} onclick={up}><ArrowUp size={18} /></button>
        </div>
        <div class="address-bar">
          <Home size={15} class="address-home" />
          {#if editingAddress}
            <form class="address-form" onsubmit={(event) => { event.preventDefault(); if (address.trim()) void navigate(address.trim()); }}><input aria-label="文件夹路径" bind:this={addressInput} bind:value={address} spellcheck="false" onkeydown={(event) => { if (event.key === 'Escape') { event.stopPropagation(); editingAddress = false; } }} /><button class="icon-button" type="submit" title="转到此路径" aria-label="转到此路径" disabled={busy}><ArrowRight size={15} /></button></form>
          {:else}
            <nav class="breadcrumbs" aria-label="路径导航">
              {#each crumbs as crumb, index}
                {#if index > 0}<ChevronRight size={13} />{/if}
                <button disabled={busy} class:current={index === crumbs.length - 1} onclick={() => navigate(crumb.path)} title={crumb.path}>{info?.places.find((p) => p.path === crumb.path)?.name ?? crumb.name}</button>
              {/each}
            </nav>
            <button class="address-edit" title={`编辑路径 (${shortcutModifier}+L)`} aria-label="编辑路径" onclick={editAddress} disabled={!directory}><Pencil size={13} /></button>
          {/if}
        </div>
        <button class="icon-button refresh-button" title="刷新 (F5)" aria-label="刷新" onclick={refresh} disabled={busy || !directory}><RotateCw size={17} class={loading ? 'spin' : ''} /></button>
        <label class="search-box"><Search size={16} /><input bind:this={searchInput} bind:value={query} placeholder="搜索当前文件夹" aria-label="搜索当前文件夹" disabled={!directory} />{#if query}<button class="clear-search" title="清除搜索" aria-label="清除搜索" onclick={() => query = ''}><X size={14} /></button>{:else}<kbd>{shortcutModifier} F</kbd>{/if}</label>
      </div>

      <div class="content-layout">
        <section class="browser" aria-label="文件浏览器">
          <div class="folder-heading">
            <div class="folder-title"><span class="eyebrow">我的空间 <span>/</span> {currentPlace?.id === 'home' ? 'HOME' : 'FILES'}</span><h1>{title}<span class="heading-dot">.</span></h1><p>{query ? `“${query}” 的筛选结果` : '让每个文件，都在合适的地方。'}</p></div>
            <button class="button primary create-button" onclick={() => openDialog('create')} disabled={busy || !directory}><FolderPlus size={17} />新建文件夹</button>
          </div>

          <div class="file-toolbar">
            <div class="file-summary">{#if selected.size}<span class="selection-label">已选择 {selected.size} 项</span><button class="toolbar-text" onclick={() => selected = new Set()}>取消选择</button>{:else}<span class="folder-summary-icon"><Folder size={15} /></span><span>{folderCount} 个文件夹<span class="summary-separator">·</span>{fileCount} 个文件</span>{/if}</div>
            <div class="view-tools">
              <div class="sort-control"><ChevronsUpDown size={14} /><select aria-label="排序方式" value={sort} onchange={(event) => { sort = event.currentTarget.value as SortKey; }}><option value="name">名称</option><option value="modified">修改时间</option><option value="size">大小</option><option value="type">类型</option></select><button class="sort-direction" title={descending ? '切换升序' : '切换降序'} aria-label={descending ? '切换升序' : '切换降序'} onclick={() => descending = !descending}>{#if descending}<ArrowDown size={13} />{:else}<ArrowUp size={13} />{/if}</button></div>
              <span class="tool-separator"></span>
              <div class="view-switch" aria-label="显示方式"><button class:active={preferences.view === 'grid'} title="图标视图" aria-label="图标视图" aria-pressed={preferences.view === 'grid'} onclick={() => preferences.view = 'grid'}><LayoutGrid size={16} /></button><button class:active={preferences.view === 'list'} title="列表视图" aria-label="列表视图" aria-pressed={preferences.view === 'list'} onclick={() => preferences.view = 'list'}><List size={17} /></button></div>
              <button class="icon-button compact inspector-toggle" class:active={preferences.inspector} title="信息面板" aria-label="信息面板" aria-pressed={preferences.inspector} onclick={() => preferences.inspector = !preferences.inspector}><PanelRight size={17} /></button>
              <div class="options-container"><button class="icon-button compact" title="更多显示选项" aria-label="更多显示选项" aria-expanded={optionsOpen} onclick={(event) => { event.stopPropagation(); optionsOpen = !optionsOpen; context = null; }}><MoreHorizontal size={19} /></button>
                {#if optionsOpen}<div class="dropdown-menu" role="menu"><button role="menuitemcheckbox" aria-checked={preferences.hidden} onclick={() => preferences.hidden = !preferences.hidden}>{#if preferences.hidden}<Eye size={16} />{:else}<EyeOff size={16} />{/if}<span>显示隐藏文件</span>{#if preferences.hidden}<Check size={14} />{/if}</button><button role="menuitem" onclick={copyPath}><Link2 size={16} /><span>复制当前路径</span></button><button role="menuitem" onclick={refresh} disabled={busy}><RotateCw size={16} /><span>刷新文件夹</span><kbd>F5</kbd></button></div>{/if}
              </div>
            </div>
          </div>

          {#if clipboard}<div class="clipboard-strip">{#if clipboard.mode === 'copy'}<Copy size={14} />{:else}<Scissors size={14} />{/if}<span>{clipboard.paths.length} 个项目等待{clipboard.mode === 'cut' ? '移动' : '复制'}</span><button onclick={paste} disabled={busy}>粘贴到此处 <ClipboardPaste size={13} /></button><button class="icon-button" title="取消剪贴操作" aria-label="取消剪贴操作" onclick={() => clipboard = null} disabled={operating}><X size={13} /></button></div>{/if}

          <div class="file-area" class:loading class:list-view={preferences.view === 'list'} bind:this={fileArea} role="group" aria-label="文件和文件夹" aria-busy={busy} oncontextmenu={(event) => contextMenu(event)}>
            {#if initialError}
              <div class="empty-state"><span class="empty-icon error"><CircleAlert size={34} strokeWidth={1.3} /></span><h2>暂时无法打开文件夹</h2><p>{initialError}</p><button class="button" onclick={() => info ? navigate(info.home) : initialize()}><RotateCw size={15} />重新尝试</button></div>
            {:else if !directory && loading}
              <div class="empty-state"><LoaderCircle class="spin" size={28} /><p>正在打开你的空间…</p></div>
            {:else if !entries.length}
              <div class="empty-state"><span class="empty-icon">{#if query}<Search size={34} strokeWidth={1.3} />{:else}<Folder size={38} strokeWidth={1.2} />{/if}</span><h2>{query ? '没有找到匹配的项目' : '留一点空间，给新的可能'}</h2><p>{query ? '试试其他名称，搜索仅在当前文件夹中进行。' : '这个文件夹还是空的，从创建一个文件夹开始吧。'}</p>{#if query}<button class="button" onclick={() => query = ''}>清除搜索</button>{:else}<button class="button" onclick={() => openDialog('create')} disabled={busy}><FolderPlus size={16} />新建文件夹</button>{/if}</div>
            {:else if preferences.view === 'grid'}
              <div class="file-grid" style={`--cell-size: ${preferences.size}px`}>
                {#each entries as entry, index (entry.path)}
                  <button class="file-card" class:selected={selected.has(entry.path)} class:cut={clipboard?.mode === 'cut' && clipboard.paths.includes(entry.path)} data-index={index} aria-label={entry.name} aria-pressed={selected.has(entry.path)} title={`${entry.name}\n${entry.isDir ? '文件夹' : formatSize(entry.size)}\n双击打开`} onclick={(event) => selectEntry(entry, event)} ondblclick={() => openEntry(entry)} oncontextmenu={(event) => contextMenu(event, entry)} disabled={operating}>
                    {#if selected.has(entry.path)}<span class="selected-check"><Check size={10} strokeWidth={3} /></span>{/if}
                    <FileIcon {entry} large />
                    <span class="file-name">{entry.name}</span><span class="file-caption">{entry.isDir ? folderLabels[entry.name] ?? '文件夹' : formatSize(entry.size)}</span>
                  </button>
                {/each}
              </div>
            {:else}
              <div class="list-header"><button onclick={() => setSort('name')}>名称{#if sort === 'name'}{#if descending}<ChevronDown size={13} />{:else}<ArrowUp size={12} />{/if}{/if}</button><button onclick={() => setSort('modified')}>修改时间</button><button onclick={() => setSort('type')}>类型</button><button onclick={() => setSort('size')}>大小</button></div>
              <div class="file-list">
                {#each entries as entry, index (entry.path)}
                  <button class="file-row" class:selected={selected.has(entry.path)} class:cut={clipboard?.mode === 'cut' && clipboard.paths.includes(entry.path)} data-index={index} aria-label={entry.name} aria-pressed={selected.has(entry.path)} onclick={(event) => selectEntry(entry, event)} ondblclick={() => openEntry(entry)} oncontextmenu={(event) => contextMenu(event, entry)} disabled={operating}>
                    <span class="row-name"><FileIcon {entry} /><span>{entry.name}</span></span><span class="row-date">{formatDate(entry.modified)}</span><span class="row-type">{fileKind(entry)}</span><span class="row-size">{entry.isDir ? '—' : formatSize(entry.size)}</span>
                  </button>
                {/each}
              </div>
            {/if}
          </div>

          <div class="browser-bottom"><span><span class="small-status-dot"></span>{query ? `筛选出 ${entries.length} 个项目` : `${entries.length} 个项目`}{#if selected.size}<span class="summary-separator">·</span>已选择 {selected.size} 项{#if selectedSize > 0}，{formatSize(selectedSize)}{/if}{/if}</span><span class="zoom-controls">{#if preferences.view === 'grid'}<LayoutGrid size={12} /><input type="range" aria-label="图标大小" min="104" max="168" step="1" value={preferences.size} oninput={(event) => preferences.size = Number(event.currentTarget.value)} /><LayoutGrid size={16} />{:else}<List size={13} /><span>详细列表</span>{/if}</span></div>
        </section>

        {#if preferences.inspector}
          <aside class="inspector" aria-label="信息面板">
            <div class="inspector-heading"><span>详细信息</span><button class="icon-button compact" title="关闭信息面板" aria-label="关闭信息面板" onclick={() => preferences.inspector = false}><X size={16} /></button></div>
            <div class="inspector-preview" class:multi={selected.size > 1}><div class="preview-orbit"></div><FileIcon entry={inspectorEntry} large />{#if selected.size > 1}<span class="multi-count">{selected.size}</span>{/if}</div>
            <div class="inspector-name"><h2>{selected.size > 1 ? `${selected.size} 个项目` : focused?.name ?? title}</h2><span class="type-badge">{selected.size > 1 ? '多项选择' : focused ? fileKind(focused) : '当前文件夹'}</span></div>
            <div class="inspector-divider"></div>
            <dl class="properties"><div><dt>位置</dt><dd class="path-value" title={focused?.path ?? directory?.path}>{selected.size > 1 ? directory?.path : focused?.path ?? directory?.path ?? '—'}</dd></div>{#if focused}<div><dt>类型</dt><dd>{fileKind(focused)}</dd></div>{#if !focused.isDir}<div><dt>大小</dt><dd>{formatSize(focused.size)}</dd></div>{/if}<div><dt>修改时间</dt><dd>{formatDate(focused.modified)}</dd></div>{:else if selected.size > 1}<div><dt>已选项目</dt><dd>{selected.size} 项</dd></div><div><dt>文件大小</dt><dd>{formatSize(selectedSize)}</dd></div>{:else}<div><dt>内容</dt><dd>{directory?.entries.filter((e) => preferences.hidden || !e.hidden).length ?? 0} 个项目</dd></div><div><dt>文件夹</dt><dd>{directory?.entries.filter((e) => e.isDir && (preferences.hidden || !e.hidden)).length ?? 0}</dd></div><div><dt>文件</dt><dd>{directory?.entries.filter((e) => !e.isDir && (preferences.hidden || !e.hidden)).length ?? 0}</dd></div>{/if}</dl>
            {#if focused}<button class="button inspector-open" onclick={() => openEntry(focused!)} disabled={busy}><ExternalLink size={15} />{focused.isDir ? '打开文件夹' : '使用默认应用打开'}<ArrowRight size={14} /></button>{/if}
            <div class="inspector-tip"><span class="tip-label"><Info size={14} />小提示</span><p>{selected.size ? `按住 ${shortcutModifier} 点选多个项目，或按 Shift 连续选择。` : '单击查看文件信息，双击打开。也可以在地址栏中输入路径。'}</p></div>
            <div class="inspector-bottom"><span class="tiny-spark">✦</span> 少一点寻找，多一点专注。</div>
          </aside>
        {/if}
      </div>

      <footer class="status-bar"><span>{#if operating}<LoaderCircle size={12} class="spin" />正在处理文件…{:else if loading}<LoaderCircle size={12} class="spin" />正在读取…{:else}<span class="status-dot"></span>一切就绪{/if}{#if directory?.skipped}<span class="skipped-label">已跳过 {directory.skipped} 个无法读取的项目</span>{/if}</span><span>{#if !desktop}浏览器演示 · 示例数据仅保存在本次会话{:else}{info?.platform === 'macos' ? 'macOS' : info?.platform === 'windows' ? 'Windows' : 'Linux'} · 本地文件系统{/if}<span class="status-divider"></span><kbd>{shortcutModifier} L</kbd> 输入路径</span></footer>
    </main>
  </div>

  {#if notice}<div class="toast" class:error={notice.error} role={notice.error ? 'alert' : 'status'}>{#if notice.error}<CircleAlert size={18} />{:else}<Check size={18} />{/if}<span>{notice.text}</span><button class="icon-button compact" title="关闭提示" aria-label="关闭提示" onclick={() => notice = null}><X size={15} /></button></div>{/if}

  {#if context}<div class="context-menu" role="menu" style={`left: ${context.x}px; top: ${context.y}px`}>
    {#if selected.size === 1}<button role="menuitem" onclick={() => { context = null; if (focused) void openEntry(focused); }}><ExternalLink size={16} /><span>打开</span><kbd>Enter</kbd></button>{/if}
    {#if selected.size}<button role="menuitem" onclick={() => copySelection('copy')}><Copy size={16} /><span>复制</span><kbd>{shortcutModifier} C</kbd></button><button role="menuitem" onclick={() => copySelection('cut')}><Scissors size={16} /><span>剪切</span><kbd>{shortcutModifier} X</kbd></button>{/if}
    <button role="menuitem" onclick={paste} disabled={!clipboard}><ClipboardPaste size={16} /><span>粘贴</span><kbd>{shortcutModifier} V</kbd></button>
    <div class="menu-divider"></div>
    <button role="menuitem" onclick={() => openDialog('create')}><FolderPlus size={16} /><span>新建文件夹</span></button>
    {#if selected.size === 1}<button role="menuitem" onclick={() => openDialog('rename')}><Pencil size={16} /><span>重命名</span><kbd>F2</kbd></button>{/if}
    <button role="menuitem" onclick={copyPath}><Link2 size={16} /><span>复制路径</span></button>
    {#if selected.size}<div class="menu-divider"></div><button role="menuitem" class="danger-text" onclick={confirmTrash}><Trash2 size={16} /><span>移入回收站</span><kbd>Del</kbd></button>{/if}
  </div>{/if}

  {#if dialog}<NameDialog mode={dialog.mode} initial={dialog.mode === 'rename' ? dialog.entry?.name : ''} busy={operating} error={dialogError} onsubmit={submitName} onclose={() => { if (!operating) dialog = null; }} />{/if}

  <dialog class="modal trash-modal" bind:this={trashDialog} oncancel={(event) => { if (operating) event.preventDefault(); }}><div class="modal-top"><span class="modal-icon danger"><Trash2 size={24} /></span><button class="icon-button" title="关闭" aria-label="关闭" onclick={() => trashDialog.close()} disabled={operating}><X size={18} /></button></div><h2>移入回收站？</h2><p>将选中的 {selected.size} 个项目移入系统回收站。你可以稍后从系统回收站中恢复。</p><div class="modal-actions"><button class="button" onclick={() => trashDialog.close()} disabled={operating}>取消</button><button class="button danger-button" onclick={moveToTrash} disabled={busy}>{#if operating}<LoaderCircle size={16} class="spin" />{/if}移入回收站</button></div></dialog>

  <dialog class="modal help-modal" bind:this={helpDialog}><div class="modal-top"><span class="modal-icon"><Keyboard size={24} /></span><button class="icon-button" title="关闭" aria-label="关闭" onclick={() => helpDialog.close()}><X size={18} /></button></div><h2>顺手，就从快捷键开始</h2><p>Pudding 0.1 · Tauri + Svelte</p><dl class="shortcut-list">{#each [[`${shortcutModifier} L`, '输入文件夹路径'], [`${shortcutModifier} F`, '搜索当前文件夹'], ['Alt ↑ / ← / →', '上一级 / 后退 / 前进'], [`${shortcutModifier} Shift N`, '新建文件夹'], [`${shortcutModifier} A`, '选择所有项目'], [`${shortcutModifier} C / X / V`, '复制 / 剪切 / 粘贴'], ['F2', '重命名'], ['Delete', '移入回收站'], ['F5', '刷新'], [`${shortcutModifier} Shift .`, '显示或隐藏隐藏文件']] as [keys, label]}<div><dt>{label}</dt><dd><kbd>{keys}</kbd></dd></div>{/each}</dl>{#if !desktop}<p class="help-demo">当前为浏览器演示。使用 <code>npm run tauri dev</code> 启动桌面版，管理真实文件。</p>{/if}</dialog>
</div>
