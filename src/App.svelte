<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { Plus, ArrowLeft, ArrowRight, ArrowUp, ArrowDown, ChevronRight, ChevronDown, Home, Monitor, FileText, Download, Image, Music2, Film, HardDrive, Folder, FolderPlus, Search, X, RotateCw, LayoutGrid, List, PanelRight, MoreHorizontal, Eye, EyeOff, Copy, Scissors, ClipboardPaste, Pencil, Trash2, ExternalLink, Check, Info, LoaderCircle, Keyboard, Sun, Moon, Link2, ChevronsUpDown, CircleAlert, Eject, Settings, Network, Terminal } from '@lucide/svelte';
  import FileIcon from './lib/FileIcon.svelte';
  import FileThumbnail from './lib/FileThumbnail.svelte';
  import NameDialog from './lib/NameDialog.svelte';
  import ConflictDialog from './lib/ConflictDialog.svelte';
  import OpenWithDialog from './lib/OpenWithDialog.svelte';
  import ImagePreview from './lib/ImagePreview.svelte';
  import ImagePreviewDialog from './lib/ImagePreviewDialog.svelte';
  import FilePreviewDialog from './lib/FilePreviewDialog.svelte';
  import FileInfoDialog from './lib/FileInfoDialog.svelte';
  import { api, desktop } from './lib/api';
  import { basename, breadcrumbs, fileKind, formatDate, formatSize, formatEntrySize, isImage, isVideo, canPreview, visibleEntries } from './lib/files';
  import type { AppInfo, Clipboard, DirectoryListing, FileEntry, SortKey, ViewMode } from './lib/types';
  import type { DragDropEvent } from '@tauri-apps/api/webview';
  import { dragClientPosition } from './lib/drag';
  import { virtualWindow } from './lib/virtual-list';

  type Preferences = { view: ViewMode; hidden: boolean; inspector: boolean; size: number; dark: boolean; terminal: string; customTerminal: string };
  function loadPreferences(): Preferences {
    const defaults: Preferences = { view: 'grid', hidden: false, inspector: true, size: 120, dark: false, terminal: 'system', customTerminal: '' };
    try {
      const saved = JSON.parse(localStorage.getItem('pudding.preferences') || '{}');
      return { view: saved.view === 'list' ? 'list' : 'grid', hidden: saved.hidden === true, inspector: saved.inspector !== false, size: Number.isFinite(saved.size) && saved.size >= 104 && saved.size <= 168 ? saved.size : 120, dark: saved.dark === true, terminal: typeof saved.terminal === 'string' ? saved.terminal : 'system', customTerminal: typeof saved.customTerminal === 'string' ? saved.customTerminal : '' };
    } catch { return defaults; }
  }

  let preferences = $state(loadPreferences());
  function loadPinnedFolders(): string[] {
    try {
      const saved: unknown = JSON.parse(localStorage.getItem('pudding.pinnedFolders') || '[]');
      return Array.isArray(saved) ? [...new Set(saved.filter((path): path is string => typeof path === 'string' && path.length > 0))] : [];
    } catch { return []; }
  }
  let pinnedFolders = $state<string[]>(loadPinnedFolders());
  type FolderDisplay = { view: ViewMode; sort: SortKey; descending: boolean };
  function loadFolderDisplays(): Record<string, FolderDisplay> {
    try {
      const saved: unknown = JSON.parse(localStorage.getItem('pudding.folderDisplays') || '{}');
      if (!saved || typeof saved !== 'object' || Array.isArray(saved)) return {};
      return Object.fromEntries(Object.entries(saved).filter(([path, value]) => {
        if (!path || !value || typeof value !== 'object') return false;
        const item = value as Record<string, unknown>;
        return (item.view === 'grid' || item.view === 'list') && ['name', 'modified', 'size', 'type'].includes(item.sort as string) && typeof item.descending === 'boolean';
      }));
    } catch { return {}; }
  }
  let folderDisplays = loadFolderDisplays();
  let view = $state<ViewMode>(preferences.view);
  let info = $state<AppInfo>();
  let directory = $state.raw<DirectoryListing>();
  let loading = $state(true);
  let operating = $state(false);
  let initialError = $state('');
  let query = $state('');
  let searchContent = $state(false);
  let searchResults = $state.raw<FileEntry[] | null>(null);
  let searchVersion = $state(0);
  let searching = $state(false);
  let scanned = $state(0);
  let searchRequest = 0;
  let searchIdentity = '';
  let directoryPath = $derived(directory?.path);
  let sort = $state<SortKey>('name');
  let descending = $state(false);
  let selected = $state(new Set<string>());
  let anchor = $state<string>();
  let history = $state<string[]>([]);
  let historyIndex = $state(-1);
  type Tab = { id: number; path: string; history: string[]; historyIndex: number };
  let tabs = $state<Tab[]>([{ id: 1, path: '', history: [], historyIndex: -1 }]);
  let activeTab = $state(1);
  let nextTabId = 2;
  let clipboard = $state<Clipboard>(null);
  let dropTarget = $state<string | null>(null);
  let dropRegion = $state<'browser' | 'sidebar' | 'breadcrumbs' | null>(null);
  let dropMenu = $state<{ x: number; y: number; sources: string[]; destination: string } | null>(null);
  let dragModifiers = { ctrl: false, shift: false };
  let shiftHeld = $state(false);
  let clipboardRequest = 0;
  let volumeRequest = 0;
  let dragGesture: { x: number; y: number; entry: FileEntry; pointerId: number } | null = null;
  let demoDrag: string[] | null = null;
  let pendingOpenDirectories = $state<string[]>([]);
  let openRequestsReady = $state(false);
  let marquee = $state<{ pointerId: number; startX: number; startY: number; x: number; y: number; active: boolean; additive: boolean; initial: string[] } | null>(null);
  let editingAddress = $state(false);
  let address = $state('');
  let addressInput = $state<HTMLInputElement>();
  let searchInput: HTMLInputElement;
  let fileArea: HTMLDivElement;
  let viewport = $state({ width: 0, height: 0, scrollTop: 0, topPadding: 0 });
  function trackFileArea(node: HTMLDivElement) {
    const measure = () => {
      const style = getComputedStyle(node);
      viewport = { width: Math.max(0, node.clientWidth - (parseFloat(style.paddingLeft) || 0) - (parseFloat(style.paddingRight) || 0)), height: node.clientHeight, scrollTop: node.scrollTop, topPadding: parseFloat(style.paddingTop) || 0 };
    };
    const observer = typeof ResizeObserver === 'undefined' ? null : new ResizeObserver(measure);
    observer?.observe(node);
    node.addEventListener('scroll', measure, { passive: true });
    measure();
    return { destroy() { observer?.disconnect(); node.removeEventListener('scroll', measure); } };
  }

  let request = 0;
  let notice = $state<{ text: string; error: boolean } | null>(null);
  let noticeTimeout: ReturnType<typeof setTimeout>;
  let dialog = $state<{ mode: 'create' | 'rename'; entry?: FileEntry } | null>(null);
  let dialogError = $state('');
  let transferConflict = $state<{ paths: string[]; resolve: (choice: 'replace' | 'keep' | 'cancel') => void } | null>(null);

  async function transferWithConflicts(paths: string[], destination: string, mode: 'copy' | 'cut', onstart: () => void = () => {}): Promise<string[] | null> {
    const conflicts = await api.transferConflicts(paths, destination);
    if (!conflicts.length) { onstart(); return api.transfer(paths, destination, mode); }
    const choice = await new Promise<'replace' | 'keep' | 'cancel'>((resolve) => { transferConflict = { paths: conflicts, resolve }; });
    transferConflict = null;
    if (choice === 'cancel') return null;
    onstart();
    return api.transfer(paths, destination, mode, choice, conflicts);
  }
  let openWithEntry = $state<FileEntry | null>(null);
  let trashDialog: HTMLDialogElement;
  let permanentDelete = $state(false);
  let helpDialog: HTMLDialogElement;
  let settingsDialog: HTMLDialogElement;
  let smbDialog: HTMLDialogElement;
  let smbAddress = $state('');
  let smbUsername = $state('');
  let smbPassword = $state('');
  let smbError = $state('');
  let smbConnecting = $state(false);
  let smbShares = $state<string[] | null>(null);
  function loadSmbHistory(): string[] {
    try {
      const saved: unknown = JSON.parse(localStorage.getItem('pudding.smbHistory') || '[]');
      return Array.isArray(saved) ? saved.filter((entry): entry is string => typeof entry === 'string' && /^smb:\/\/[^/@\s]+(?:\/[^?#@]*)?$/i.test(entry)).slice(0, 10) : [];
    } catch { return []; }
  }
  let smbHistory = $state(loadSmbHistory());
  let context = $state<{ x: number; y: number } | null>(null);
  let sugarAvailable = $state(false);
  let sugarMenuOpen = $state(false);
  let sugarRequest = 0;
  let optionsOpen = $state(false);
  let preview = $state<FileEntry | null>(null);

  const placeIcons: Record<string, typeof Home> = { home: Home, desktop: Monitor, documents: FileText, downloads: Download, pictures: Image, music: Music2, videos: Film };
  const folderLabels: Record<string, string> = { Desktop: '桌面', Documents: '文档', Downloads: '下载', Music: '音乐', Pictures: '图片', Videos: '视频', Projects: '项目', Archive: '归档' };
  const shortcutModifier = /Mac|iPhone|iPad/.test(navigator.platform) ? '⌘' : 'Ctrl';

  let entries = $derived(visibleEntries(query.trim() && searchResults !== null ? searchResults : directory?.entries ?? [], query.trim() && searchResults !== null ? '' : query, preferences.hidden, sort, descending));
  let windowed = $derived(virtualWindow(entries.length, view, preferences.size, viewport.width, viewport.height, viewport.scrollTop, viewport.topPadding));
  let renderedEntries = $derived(entries.slice(windowed.start, windowed.end));
  let selectedEntries = $derived(entries.filter((entry) => selected.has(entry.path)));
  let focused = $derived(selectedEntries.length === 1 ? selectedEntries[0] : undefined);
  let pinPath = $derived(selected.size === 0 ? directory?.path : focused?.isDir ? focused.path : undefined);
  let archiveSelected = $derived(focused && !focused.isDir && /\.(?:zip|7z|tar|tar\.gz|tar\.bz2|tar\.xz|tgz|tbz2?|txz)$/i.test(focused.name));
  let currentPlace = $derived(info?.places.find((place) => place.path === directory?.path));
  let title = $derived(currentPlace?.name ?? (directory ? basename(directory.path) : '主文件夹'));
  let folderCount = $derived(entries.filter((e) => e.isDir).length);
  let fileCount = $derived(entries.length - folderCount);
  let crumbs = $derived(directory ? breadcrumbs(directory.path) : []);
  let busy = $derived(loading || operating);
  let selectedSizeKnown = $derived(selectedEntries.every((entry) => !entry.metadataPending && !entry.metadataUnavailable));
  let selectedSize = $derived(selectedEntries.reduce((total, entry) => total + (entry.isDir ? 0 : entry.size), 0));
  let inspectorEntry = $derived(focused ?? { name: title, path: directory?.path ?? '', isDir: true, isSymlink: false, size: 0, modified: null, hidden: false, extension: '' });

  let metadataRequest = $state<{ path: string; id: number } | null>(null);
  let nextMetadataRequest = 0;
  $effect(() => {
    const current = directory;
    const all = sort === 'size' || sort === 'modified';
    const candidates = all ? [...renderedEntries, ...entries] : renderedEntries;
    if (!current || metadataRequest?.path === current.path) return;
    const seen = new Set<string>();
    const pending = candidates.filter((entry) => entry.metadataPending && !seen.has(entry.path) && !!seen.add(entry.path)).slice(0, 32);
    if (!pending.length) return;
    const id = ++nextMetadataRequest;
    metadataRequest = { path: current.path, id };
    void api.readEntryDetails(current.path, pending.map((entry) => entry.name)).then((details) => {
      if (directory !== current) return;
      const found = new Map(details.map((entry) => [entry.path, entry]));
      const requested = new Set(pending.map((entry) => entry.path));
      directory = { ...current, entries: current.entries.map((entry) => found.get(entry.path) ?? (requested.has(entry.path) ? { ...entry, metadataPending: false, metadataUnavailable: true } : entry)) };
    }).catch(() => {
      if (directory !== current) return;
      const requested = new Set(pending.map((entry) => entry.path));
      directory = { ...current, entries: current.entries.map((entry) => requested.has(entry.path) ? { ...entry, metadataPending: false, metadataUnavailable: true } : entry) };
    }).finally(() => { if (metadataRequest?.id === id) metadataRequest = null; });
  });

  $effect(() => {
    try { localStorage.setItem('pudding.preferences', JSON.stringify(preferences)); } catch { /* Optional persistence. */ }
  });
  $effect(() => {
    try { localStorage.setItem('pudding.pinnedFolders', JSON.stringify(pinnedFolders)); } catch { /* Optional persistence. */ }
  });

  // Filtering must also update selection, so hidden files cannot be operated on accidentally.
  $effect(() => {
    if (searching) return;
    const visiblePaths = new Set(entries.map((e) => e.path));
    if ([...selected].some((path) => !visiblePaths.has(path))) selected = new Set([...selected].filter((path) => visiblePaths.has(path)));
  });

  $effect(() => {
    const path = directoryPath;
    const term = query.trim();
    const hidden = preferences.hidden;
    const content = searchContent;
    searchVersion;
    let cancelled = false;
    const searchId = ++searchRequest;
    scanned = 0;
    const identity = JSON.stringify([path, term, hidden, content]);
    if (identity !== searchIdentity) searchResults = null;
    searchIdentity = identity;
    searching = !!(path && term && desktop);
    if (path && term) {
      const timer = setTimeout(() => {
        void api.search(path, term, hidden, content, searchId).then((results) => { if (!cancelled) { searchResults = results; searching = false; } }).catch((error) => { if (!cancelled) { searching = false; notify(errorMessage(error), true); } });
      }, 250);
      return () => { cancelled = true; clearTimeout(timer); };
    }
  });

  $effect(() => {
    if (!openRequestsReady || busy || !pendingOpenDirectories.length) return;
    const [path, ...others] = pendingOpenDirectories;
    pendingOpenDirectories = [];
    void navigate(path);
    for (const other of others) void api.openWindow(other).catch((error) => notify(errorMessage(error), true));
  });

  onMount(() => {
    void syncClipboard();
    let disposed = false;
    let unlisten: (() => void) | undefined;
    let unlistenChanges: (() => void) | undefined;
    let unlistenProgress: (() => void) | undefined;
    let unlistenOpen: (() => void) | undefined;
    async function pullOpenDirectories() {
      try {
        const paths = await api.takeOpenDirectories();
        if (!disposed && paths.length) pendingOpenDirectories = [...pendingOpenDirectories, ...paths];
      } catch (error) { if (!disposed) notify(errorMessage(error), true); }
    }
    void (async () => {
      try {
        const cleanup = await api.listenOpenDirectories(() => void pullOpenDirectories());
        if (disposed) { cleanup(); return; }
        unlistenOpen = cleanup;
        await pullOpenDirectories();
      } catch (error) { if (!disposed) notify(errorMessage(error), true); }
      if (!disposed) { await initialize(); if (!disposed) openRequestsReady = true; }
    })();
    void api.listenSearchProgress(([id, count]) => { if (id === searchRequest) scanned = count; }).then((cleanup) => { if (disposed) cleanup(); else unlistenProgress = cleanup; }).catch((error) => notify(errorMessage(error), true));
    const volumeTimer = desktop ? setInterval(() => void refreshVolumes(), 5000) : undefined;
    let changeTimer: ReturnType<typeof setTimeout>;
    function applyChanges() {
      if (busy) { changeTimer = setTimeout(applyChanges, 300); return; }
      searchVersion++; refresh();
    }
    void api.listenChanges(() => {
      clearTimeout(changeTimer);
      changeTimer = setTimeout(applyChanges, 300);
    }).then((cleanup) => { if (disposed) cleanup(); else unlistenChanges = cleanup; }).catch((error) => notify(errorMessage(error), true));
    void api.listenDrop(nativeDrop).then((cleanup) => {
      if (disposed) cleanup(); else unlisten = cleanup;
    }).catch((error) => notify(errorMessage(error), true));
    return () => { disposed = true; unlisten?.(); unlistenChanges?.(); unlistenProgress?.(); unlistenOpen?.(); clearInterval(volumeTimer); volumeRequest++; clearTimeout(changeTimer); clipboardRequest += 1; request += 1; clearTimeout(noticeTimeout); };
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
      void refreshVolumes();
      const explicitPath = new URLSearchParams(window.location.search).get('path');
      const restoredPath = explicitPath ? null : await api.startupDirectory().catch(() => null);
      await navigate(explicitPath || restoredPath || info.home);
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
      if (directory?.path !== result.path) {
        const saved = Object.hasOwn(folderDisplays, result.path) ? folderDisplays[result.path] : undefined;
        view = saved?.view ?? preferences.view;
        sort = saved?.sort ?? 'name';
        descending = saved?.descending ?? false;
      }
      directory = result;
      void api.setWindowDirectory(result.path).catch(() => {});
      if (!refresh) void api.watch(result.path).catch((error) => notify(errorMessage(error), true));
      editingAddress = false;
      initialError = '';
      if (!refresh) {
        query = '';
        selected = new Set();
        anchor = undefined;
      } else if (!query.trim()) selected = new Set([...selected].filter((p) => result.entries.some((e) => e.path === p)));
      if (index !== undefined) historyIndex = index;
      else if (!refresh && history[historyIndex] !== result.path) {
        history = [...history.slice(0, historyIndex + 1), result.path];
        historyIndex = history.length - 1;
      }
      tabs = tabs.map((tab) => tab.id === activeTab ? { ...tab, path: result.path, history: [...history], historyIndex } : tab);
      if (fileArea && !refresh) { fileArea.scrollTop = 0; viewport.scrollTop = 0; }
    } catch (error) {
      if (id !== request) return;
      if (!directory) initialError = errorMessage(error);
      else notify(errorMessage(error), true);
    } finally { if (id === request) loading = false; }
  }

  function switchTab(id: number) {
    if (busy || id === activeTab) return;
    const tab = tabs.find((item) => item.id === id);
    if (!tab?.path) return;
    activeTab = id;
    history = [...tab.history];
    historyIndex = tab.historyIndex;
    query = '';
    selected = new Set();
    anchor = undefined;
    preview = null;
    void api.watch(tab.path).catch((error) => notify(errorMessage(error), true));
    void navigate(tab.path, tab.historyIndex, true);
  }

  function newTab(path = directory?.path) {
    if (busy || !path) return;
    const id = nextTabId++;
    tabs = [...tabs, { id, path, history: [path], historyIndex: 0 }];
    switchTab(id);
  }

  async function openInWindow(path: string) {
    context = null;
    try { await api.openWindow(path); }
    catch (error) { notify(errorMessage(error), true); }
  }

  function closeTab(id: number) {
    if (busy || tabs.length <= 1) return;
    const index = tabs.findIndex((tab) => tab.id === id);
    if (index < 0) return;
    if (id === activeTab) switchTab(tabs[index === tabs.length - 1 ? index - 1 : index + 1].id);
    tabs = tabs.filter((tab) => tab.id !== id);
  }

  async function refreshVolumes() {
    const id = ++volumeRequest;
    try {
      const roots = await api.volumes();
      if (id === volumeRequest && info) info = { ...info, roots };
    } catch { /* Keep the last known devices if the system is temporarily unavailable. */ }
  }

  function insideVolume(path: string, root: string): boolean {
    const normalize = (value: string) => {
      const normalized = value.replace(/\\/g, '/').replace(/\/$/, '');
      return info?.platform === 'windows' ? normalized.toLowerCase() : normalized;
    };
    const target = normalize(path), prefix = normalize(root);
    return target === prefix || target.startsWith(`${prefix}/`);
  }

  async function ejectVolume(root: { path: string; name: string }) {
    if (!info || busy) return;
    const home = info.home;
    operating = true;
    try {
      await api.ejectVolume(root.path);
      const sanitize = (tab: Tab): Tab => {
        const remaining = tab.history.filter((path) => !insideVolume(path, root.path));
        const before = tab.history.slice(0, tab.historyIndex + 1).filter((path) => !insideVolume(path, root.path)).length;
        return insideVolume(tab.path, root.path)
          ? { ...tab, path: home, history: [home], historyIndex: 0 }
          : { ...tab, history: remaining.length ? remaining : [tab.path], historyIndex: Math.max(0, before - 1) };
      };
      tabs = tabs.map(sanitize);
      const active = tabs.find((tab) => tab.id === activeTab);
      if (active) { history = [...active.history]; historyIndex = active.historyIndex; }
      const wasCurrent = !!directory && insideVolume(directory.path, root.path);
      operating = false;
      if (wasCurrent) await navigate(home);
      await refreshVolumes();
      notify(`已推出「${root.name}」`);
    } catch (error) { notify(errorMessage(error), true); }
    finally { operating = false; }
  }

  async function connectSmb(event: SubmitEvent) {
    event.preventDefault();
    if (smbConnecting) return;
    smbConnecting = true;
    smbError = '';
    try {
      let address = smbAddress.trim();
      let url: URL;
      try { url = new URL(address); } catch { throw new Error('请输入有效的 SMB 地址，例如 smb://server'); }
      if (url.protocol !== 'smb:' || !url.hostname || url.username || url.password || url.search || url.hash) throw new Error('请输入不含账户信息的 SMB 地址');
      if (!url.pathname || url.pathname === '/') {
        smbShares = await api.listSmbShares(address, smbUsername.trim(), smbPassword);
        if (!smbShares.length) smbError = '服务器没有返回可用的共享文件夹。';
        return;
      }
      const paths = await api.connectSmb(address, smbUsername.trim(), smbPassword);
      smbHistory = [address, ...smbHistory.filter((item) => item !== address)].slice(0, 10);
      try { localStorage.setItem('pudding.smbHistory', JSON.stringify(smbHistory)); } catch { /* Optional persistence. */ }
      smbDialog.close();
      await refreshVolumes();
      await navigate(paths[0]);
    } catch (error) { smbError = errorMessage(error); }
    finally { smbConnecting = false; }
  }

  function refresh() { if (directory && !busy) { void refreshVolumes(); void navigate(directory.path, undefined, true); } }
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
    else if (isImage(entry)) previewImage(entry);
    else await openExternal(entry);
  }

  async function openExternal(entry: FileEntry) {
    try { await api.open(entry.path); }
    catch (error) { notify(errorMessage(error), true); }
  }

  function openWith(entry: FileEntry) {
    context = null;
    openWithEntry = entry;
  }

  async function openTerminal(path: string) {
    context = null;
    optionsOpen = false;
    try { await api.openTerminal(path, preferences.terminal, preferences.customTerminal); }
    catch (error) { notify(errorMessage(error), true); }
  }

  async function chooseTerminal() {
    try {
      const path = await api.chooseTerminal();
      if (path) { preferences.customTerminal = path; preferences.terminal = 'custom'; }
    } catch (error) { notify(errorMessage(error), true); }
  }

  function previewImage(entry: FileEntry) {
    if (busy) return;
    context = null;
    optionsOpen = false;
    preview = entry;
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

  async function syncClipboard() {
    const id = ++clipboardRequest;
    try {
      const value = await api.readClipboard();
      if (id === clipboardRequest) clipboard = value;
    } catch { if (id === clipboardRequest) clipboard = null; }
  }

  async function cancelClipboard() {
    if (!clipboard || busy) return;
    const expected = $state.snapshot(clipboard);
    ++clipboardRequest;
    try { await api.clearClipboard(expected); await syncClipboard(); }
    catch (error) { notify(errorMessage(error), true); }
  }

  async function copySelection(mode: 'copy' | 'cut') {
    if (!selected.size || busy) return;
    const operation = { paths: [...selected], mode };
    context = null;
    operating = true;
    ++clipboardRequest;
    try {
      await api.writeClipboard(operation);
      clipboard = operation;
      notify(`已${mode === 'cut' ? '剪切' : '复制'} ${operation.paths.length} 个项目，请到目标文件夹粘贴`);
    } catch (error) { notify(errorMessage(error), true); }
    finally { operating = false; }
  }

  async function paste() {
    if (!directory || busy) return;
    context = null;
    operating = true;
    const destination = directory.path;
    let paths: string[] = [];
    let started = false;
    try {
      // Read at paste time so external changes never use stale paths.
      const operation = await api.readClipboard();
      clipboard = operation;
      if (!operation) { notify('剪贴板中没有可粘贴的文件'); return; }
      try {
        const result = await transferWithConflicts([...operation.paths], destination, operation.mode, () => { started = true; });
        if (result === null) return;
        paths = result;
        notify(`已${operation.mode === 'cut' ? '移动' : '复制'} ${paths.length} 个项目`);
      } finally {
        // A batch may partially succeed. Clear stale cut paths only if the
        // clipboard has not been replaced by another application.
        if (operation.mode === 'cut' && started) await api.clearClipboard(operation);
      }
    } catch (error) {
      notify(errorMessage(error), true);
    } finally {
      await syncClipboard();
      operating = false;
      await navigate(destination, undefined, true);
      if (paths.length) selected = new Set(paths);
    }
  }

  function dropPath(x: number, y: number): string | null {
    dropRegion = null;
    if (busy || preview || document.querySelector('dialog[open]')) return null;
    const target = document.elementFromPoint(x, y)?.closest<HTMLElement>('[data-drop-path]');
    if (!target) return null;
    dropRegion = target.closest('.places') ? 'sidebar' : target.closest('.breadcrumbs') ? 'breadcrumbs' : 'browser';
    return target.dataset.dropPath ?? null;
  }

  function nativeDrop(event: DragDropEvent) {
    if (event.type === 'leave') { dropTarget = null; return; }
    const position = dragClientPosition(event.position, info?.platform ?? '', window.devicePixelRatio);
    const target = dropPath(position.x, position.y);
    dropTarget = event.type === 'drop' ? null : target;
    if (event.type === 'drop' && target) void chooseDrop(event.paths, target, position, { ...dragModifiers }, true);
  }

  async function chooseDrop(sources: string[], destination: string, position: { x: number; y: number }, modifiers: { ctrl: boolean; shift: boolean }, native = false) {
    if (!sources.length || busy) return;
    dropMenu = null;
    if (native) {
      try { modifiers = await api.dropModifiers(); } catch { /* Fall back to the last keyboard state. */ }
    }
    if (modifiers.ctrl !== modifiers.shift) {
      void receiveDrop(sources, destination, modifiers.ctrl ? 'copy' : 'cut');
    } else {
      dropMenu = { sources, destination, x: Math.max(8, Math.min(position.x, innerWidth - 220)), y: Math.max(8, Math.min(position.y, innerHeight - 120)) };
    }
  }

  function confirmDrop(mode: 'copy' | 'cut') {
    const pending = dropMenu;
    dropMenu = null;
    if (pending) void receiveDrop(pending.sources, pending.destination, mode);
  }

  async function receiveDrop(sources: string[], destination: string, mode: 'copy' | 'cut') {
    if (!sources.length || !directory || busy) return;
    const current = directory.path;
    operating = true;
    let paths: string[] = [];
    try {
      const result = await transferWithConflicts(sources, destination, mode);
      if (result === null) return;
      paths = result;
      notify(`已${mode === 'cut' ? '移动' : '复制'} ${paths.length} 个项目到「${basename(destination) || destination}」`);
    } catch (error) { notify(errorMessage(error), true); }
    finally {
      operating = false;
      await navigate(current, undefined, true);
      if (destination === current && paths.length) selected = new Set(paths);
    }
  }

  function beginDrag(event: PointerEvent, entry: FileEntry) {
    if (desktop && event.button === 0 && !busy) dragGesture = { x: event.clientX, y: event.clientY, entry, pointerId: event.pointerId };
  }

  function beginMarquee(event: PointerEvent) {
    if (busy || event.button !== 0 || event.pointerType === 'touch' || !(event.target instanceof Element) || event.target.closest('button, input, select, textarea, a')) return;
    const bounds = fileArea.getBoundingClientRect();
    if (event.clientX < bounds.left || event.clientX > bounds.right || event.clientY < bounds.top || event.clientY > bounds.bottom || (fileArea.clientWidth > 0 && event.clientX >= bounds.left + fileArea.clientWidth)) return;
    event.preventDefault();
    marquee = { pointerId: event.pointerId, startX: event.clientX, startY: event.clientY, x: event.clientX, y: event.clientY, active: false, additive: event.ctrlKey || event.metaKey || event.shiftKey, initial: [...selected] };
  }

  function updateMarquee(event: PointerEvent) {
    if (!marquee || event.pointerId !== marquee.pointerId) return;
    if (!(event.buttons & 1) || busy) { marquee = null; return; }
    const bounds = fileArea.getBoundingClientRect();
    marquee.x = Math.max(bounds.left, Math.min(bounds.right, event.clientX));
    marquee.y = Math.max(bounds.top, Math.min(bounds.bottom, event.clientY));
    if (!marquee.active && Math.hypot(marquee.x - marquee.startX, marquee.y - marquee.startY) < 5) return;
    marquee.active = true;
    const left = Math.min(marquee.startX, marquee.x), right = Math.max(marquee.startX, marquee.x);
    const top = Math.min(marquee.startY, marquee.y), bottom = Math.max(marquee.startY, marquee.y);
    const next = new Set(marquee.additive ? marquee.initial : []);
    let last: string | undefined;
    fileArea.querySelectorAll<HTMLElement>('[data-index]').forEach((element) => {
      const rect = element.getBoundingClientRect();
      const entry = entries[Number(element.dataset.index)];
      if (entry && rect.right > left && rect.left < right && rect.bottom > top && rect.top < bottom) {
        next.add(entry.path);
        last = entry.path;
      }
    });
    selected = next;
    if (last) anchor = last;
  }

  function endMarquee(event: PointerEvent, cancelled = false) {
    if (!marquee || event.pointerId !== marquee.pointerId) return;
    if (cancelled) selected = new Set(marquee.initial);
    else if (!marquee.active && !marquee.additive) { selected = new Set(); anchor = undefined; }
    marquee = null;
  }

  function pointerMove(event: PointerEvent) {
    updateMarquee(event);
    const gesture = dragGesture;
    if (!gesture || event.pointerId !== gesture.pointerId) return;
    if (!(event.buttons & 1) || busy) { dragGesture = null; return; }
    if (Math.hypot(event.clientX - gesture.x, event.clientY - gesture.y) < 6) return;
    dragGesture = null;
    if (!selected.has(gesture.entry.path)) selected = new Set([gesture.entry.path]);
    void api.startDrag([...selected]).catch((error) => notify(errorMessage(error), true));
  }

  function demoDragStart(event: DragEvent, entry: FileEntry) {
    if (desktop || busy) { event.preventDefault(); return; }
    if (!selected.has(entry.path)) selected = new Set([entry.path]);
    demoDrag = [...selected];
    event.dataTransfer?.setData('application/x-pudding-files', JSON.stringify(demoDrag));
    if (event.dataTransfer) event.dataTransfer.effectAllowed = 'copyMove';
  }

  function demoDragOver(event: DragEvent) {
    if (!demoDrag) return;
    const target = dropPath(event.clientX, event.clientY);
    dropTarget = target;
    if (target) { event.preventDefault(); if (event.dataTransfer) event.dataTransfer.dropEffect = event.shiftKey && !event.ctrlKey ? 'move' : 'copy'; }
  }

  function demoDrop(event: DragEvent) {
    if (!demoDrag) return;
    event.preventDefault();
    const target = dropPath(event.clientX, event.clientY);
    const paths = demoDrag;
    demoDrag = null;
    dropTarget = null;
    if (target) void chooseDrop(paths, target, { x: event.clientX, y: event.clientY }, { ctrl: event.ctrlKey, shift: event.shiftKey });
  }

  function confirmTrash(permanent = false) {
    if (!selected.size || busy) return;
    permanentDelete = permanent;
    context = null;
    trashDialog.showModal();
  }

  async function moveToTrash() {
    if (!directory || busy || !selected.size) return;
    const paths = [...selected];
    operating = true;
    try {
      if (permanentDelete) await api.delete(paths);
      else await api.trash(paths);
      clipboard = null;
      selected = new Set();
      notify(permanentDelete ? `已永久删除 ${paths.length} 个项目` : `已将 ${paths.length} 个项目移入回收站`);
    } catch (error) { notify(errorMessage(error), true); }
    finally {
      operating = false;
      trashDialog.close();
      await navigate(directory.path, undefined, true);
    }
  }

  async function copyPath(scope: 'selection' | 'directory') {
    context = null;
    optionsOpen = false;
    const path = scope === 'selection' ? selectedEntries.map((e) => e.path).join('\n') : directory?.path;
    if (!path) return;
    try { await api.writeText(path); await syncClipboard(); notify('路径已复制到剪贴板'); }
    catch { notify('无法访问系统剪贴板，可在地址栏中手动复制路径。', true); }
  }

  async function runSugar(action: string) {
    const paths = selected.size ? [...selected] : directory ? [directory.path] : [];
    context = null;
    if (!paths.length || busy) return;
    operating = true;
    try { await api.sugarArchive(action, paths); notify(action === 'custom' ? '已打开 Sugar' : action === 'compress' ? '压缩完成' : '解压完成'); }
    catch (error) { notify(errorMessage(error), true); }
    finally { operating = false; if (action !== 'custom') refresh(); }
  }

  function contextMenu(event: MouseEvent, entry?: FileEntry) {
    event.preventDefault();
    event.stopPropagation();
    if (busy) return;
    shiftHeld = event.shiftKey;
    if (entry && !selected.has(entry.path)) selected = new Set([entry.path]);
    if (!entry) selected = new Set();
    optionsOpen = false;
    sugarAvailable = false;
    sugarMenuOpen = false;
    const request = ++sugarRequest;
    if (directory && desktop) void api.sugarAvailable().then((available) => { if (request === sugarRequest && context) sugarAvailable = available; }).catch(() => {});
    context = { x: Math.max(8, Math.min(event.clientX, innerWidth - 220)), y: Math.max(8, Math.min(event.clientY, innerHeight - 450)) };
  }

  function saveFolderDisplay() {
    if (!directory) return;
    folderDisplays = { ...folderDisplays, [directory.path]: { view, sort, descending } };
    try { localStorage.setItem('pudding.folderDisplays', JSON.stringify(folderDisplays)); } catch { /* Optional persistence. */ }
  }

  function setView(value: ViewMode) { view = value; saveFolderDisplay(); }
  function setSort(key: SortKey) {
    if (sort === key) descending = !descending;
    else { sort = key; descending = key === 'modified'; }
    saveFolderDisplay();
  }
  function clearFolderDisplays() {
    folderDisplays = {};
    try { localStorage.removeItem('pudding.folderDisplays'); } catch { /* Optional persistence. */ }
    view = preferences.view;
    sort = 'name';
    descending = false;
    notify('已清理文件夹显示与排序记录');
  }

  function mouseNavigation(event: MouseEvent) {
    if (event.button !== 3 && event.button !== 4) return;
    // Suppress the webview's own history navigation, even at history boundaries.
    event.preventDefault();
    if (document.querySelector('dialog[open]') || busy) return;
    if (event.button === 3) back(); else forward();
  }

  function keydown(event: KeyboardEvent) {
    dragModifiers = { ctrl: event.ctrlKey, shift: event.shiftKey };
    shiftHeld = event.shiftKey;
    if (event.key === 'Escape') {
      if (marquee) { selected = new Set(marquee.initial); marquee = null; }
      dropMenu = null;
      context = null;
      optionsOpen = false;
      if (editingAddress) editingAddress = false;
      return;
    }
    if (preview && event.code === 'Space' && !event.altKey && !event.ctrlKey && !event.metaKey) {
      event.preventDefault();
      if (!event.repeat) preview = null;
      return;
    }
    if (document.querySelector('dialog[open]')) return;
    const target = event.target as HTMLElement;
    const modifier = event.ctrlKey || event.metaKey;
    if (modifier && event.key.toLowerCase() === 't') { event.preventDefault(); newTab(); return; }
    if (modifier && event.key.toLowerCase() === 'w') { event.preventDefault(); closeTab(activeTab); return; }
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
    else if (key === 'delete') { event.preventDefault(); confirmTrash(event.shiftKey); }
    else if (event.code === 'Space' && !event.repeat && !event.altKey && !event.ctrlKey && !event.metaKey && focused && (target.closest('.file-area') || target === document.body)) { event.preventDefault(); previewImage(focused); }
    else if (key === 'enter' && focused && (target.closest('.file-area') || target === document.body)) { event.preventDefault(); void openEntry(focused); }
    else if (['arrowleft', 'arrowright', 'arrowup', 'arrowdown'].includes(key) && entries.length && (target.closest('.file-area') || target === document.body)) {
      event.preventDefault();
      const columns = windowed.columns;
      const offset = key === 'arrowleft' ? -1 : key === 'arrowright' ? 1 : key === 'arrowup' ? -columns : columns;
      const index = entries.findIndex((e) => e.path === anchor);
      const next = index < 0 ? 0 : Math.min(entries.length - 1, Math.max(0, index + offset));
      anchor = entries[next].path;
      selected = new Set([anchor]);
      if (entries.length > 300 && viewport.height > 0) {
        const top = windowed.top + Math.floor(next / columns) * windowed.rowStep;
        const bottom = top + windowed.rowStep;
        if (top < fileArea.scrollTop + (view === 'list' ? 37 : 0)) fileArea.scrollTop = Math.max(0, top - (view === 'list' ? 37 : 0));
        else if (bottom > fileArea.scrollTop + fileArea.clientHeight) fileArea.scrollTop = bottom - fileArea.clientHeight;
        viewport.scrollTop = fileArea.scrollTop;
      }
      void tick().then(() => fileArea.querySelector<HTMLButtonElement>(`[data-index="${next}"]`)?.focus());
    }
  }
</script>

<svelte:window onmousedown={mouseNavigation} onauxclick={(event) => { if (event.button === 3 || event.button === 4) event.preventDefault(); }} onfocus={() => { void syncClipboard(); void refreshVolumes(); }} onpointermove={pointerMove} onpointerup={(event) => { dragGesture = null; endMarquee(event); }} onpointercancel={(event) => { dragGesture = null; endMarquee(event, true); }} ondragover={demoDragOver} ondrop={demoDrop} ondragend={() => { demoDrag = null; dropTarget = null; }} ondragleave={(event) => { if (!event.relatedTarget) dropTarget = null; }} onkeydown={keydown} onkeyup={(event) => { dragModifiers = { ctrl: event.ctrlKey, shift: event.shiftKey }; shiftHeld = event.shiftKey; }} onclick={() => { context = null; optionsOpen = false; dropMenu = null; }} onblur={() => { context = null; optionsOpen = false; dragGesture = null; dropTarget = null; shiftHeld = false; if (marquee) selected = new Set(marquee.initial); marquee = null; }} onresize={() => context = null} />

<div class="app-shell" data-theme={preferences.dark ? 'dark' : 'light'}>
  <header class="app-header">
    <a class="brand" href="#home" onclick={(event) => { event.preventDefault(); if (info && !busy) void navigate(info.home); }} aria-label="Pudding 主文件夹">
      <span class="brand-mark"><svg viewBox="0 0 32 32" fill="none" aria-hidden="true"><path d="M6 11a2 2 0 0 1 2-2h7l3 3h7a2 2 0 0 1 2 2v10a2 2 0 0 1-2 2H8a2 2 0 0 1-2-2V11Z" fill="white" opacity=".95"/><path d="M6 16h21v8a2 2 0 0 1-2 2H8a2 2 0 0 1-2-2v-8Z" fill="#b9d2ff"/></svg></span>
      <span>Pudding</span>
    </a>
      <div class="tab-bar" aria-label="文件夹标签页">
        <div class="tab-list" role="tablist" aria-label="文件夹标签页">
          {#each tabs as tab (tab.id)}
            <div class="tab" class:active={tab.id === activeTab}>
              <button class="tab-select" role="tab" aria-selected={tab.id === activeTab} title={tab.path} disabled={busy} onclick={() => switchTab(tab.id)}><Folder size={14} /><span>{info?.places.find((place) => place.path === tab.path)?.name ?? (tab.path ? basename(tab.path) : '文件夹')}</span></button>
              {#if tabs.length > 1}<button class="tab-close" aria-label="关闭标签页" title="关闭标签页" disabled={busy} onclick={() => closeTab(tab.id)}><X size={13} /></button>{/if}
            </div>
          {/each}
        </div>
        <button class="icon-button tab-add" title="新建标签页" aria-label="新建标签页" disabled={busy || !directory} onclick={() => newTab()}><Plus size={17} /></button>
      </div>
    <div class="header-actions">
      {#if !desktop}<span class="demo-badge" title="浏览器中使用示例数据；桌面版连接真实文件系统">演示模式</span>{:else}<span class="desktop-badge"><span class="status-dot"></span>本地工作空间</span>{/if}
      <button class="icon-button" title="设置" aria-label="设置" onclick={() => settingsDialog.showModal()}><Settings size={18} /></button>
      <button class="icon-button" title="快捷键与帮助" aria-label="快捷键与帮助" onclick={() => helpDialog.showModal()}><Keyboard size={18} /></button>
      <button class="icon-button" title={preferences.dark ? '切换浅色外观' : '切换深色外观'} aria-label={preferences.dark ? '切换浅色外观' : '切换深色外观'} onclick={() => preferences.dark = !preferences.dark}>{#if preferences.dark}<Sun size={18} />{:else}<Moon size={18} />{/if}</button>
    </div>
  </header>

  <div class="workspace">
    <aside class="sidebar" aria-label="位置侧栏">
      <div class="sidebar-scroll">
        <div class="section-label places-heading">常用位置<button class="icon-button compact" title={pinPath && pinnedFolders.includes(pinPath) ? '文件夹已固定' : '固定当前或选中的文件夹'} aria-label="固定当前或选中的文件夹" disabled={!pinPath || pinnedFolders.includes(pinPath)} onclick={() => { if (pinPath && !pinnedFolders.includes(pinPath)) pinnedFolders = [...pinnedFolders, pinPath]; }}><Plus size={16} /></button></div>
        <nav class="places" aria-label="常用位置">
          {#each info?.places ?? [] as place}
            {@const PlaceIcon = placeIcons[place.id] ?? Folder}
            <button data-drop-path={place.path} class:drop-target={dropRegion === 'sidebar' && dropTarget === place.path} class:active={directory?.path === place.path} onclick={() => navigate(place.path)} disabled={operating} aria-current={directory?.path === place.path ? 'page' : undefined} title={place.path}><PlaceIcon size={18} strokeWidth={1.8} /><span>{place.name}</span>{#if place.id === 'home'}<span class="home-key">⌂</span>{/if}</button>
          {/each}
          {#each pinnedFolders as path (path)}
            <div class="device-row"><button class="device-open" data-drop-path={path} class:drop-target={dropRegion === 'sidebar' && dropTarget === path} class:active={directory?.path === path} title={path} aria-label={`打开固定文件夹 ${basename(path) || path}`} aria-current={directory?.path === path ? 'page' : undefined} onclick={() => navigate(path)} disabled={busy}><Folder size={18} strokeWidth={1.8} /><span>{info?.places.find((place) => place.path === path)?.name ?? (basename(path) || path)}</span></button><button class="device-eject" title="取消固定" aria-label={`取消固定 ${basename(path) || path}`} onclick={() => pinnedFolders = pinnedFolders.filter((item) => item !== path)}><X size={15} /></button></div>
          {/each}
          {#if !info}{#each Array(7) as _}<div class="sidebar-skeleton"></div>{/each}{/if}
        </nav>
        <div class="sidebar-divider"></div>
        <div class="section-label">设备与磁盘<span>DEVICES</span></div>
        <nav class="places devices" aria-label="设备与磁盘">
          {#each info?.roots ?? [] as root}
            <div class="device-row"><button class="device-open" data-drop-path={root.path} class:drop-target={dropRegion === 'sidebar' && dropTarget === root.path} class:active={directory?.path === root.path} onclick={() => navigate(root.path)} disabled={operating} title={root.path}><HardDrive size={18} strokeWidth={1.8} /><span>{root.name}</span></button>{#if root.ejectable}<button class="device-eject" title={`推出 ${root.name}`} aria-label={`推出 ${root.name}`} onclick={() => ejectVolume(root)} disabled={busy}><Eject size={15} /></button>{/if}</div>
          {/each}
          <button onclick={() => { smbError = ''; smbShares = null; smbDialog.showModal(); }} disabled={!desktop || !['macos', 'linux'].includes(info?.platform ?? '')} title={!desktop ? '请在桌面版中连接 SMB' : !['macos', 'linux'].includes(info?.platform ?? '') ? '当前仅支持 macOS 和 Linux' : '连接 SMB 共享'}><Network size={18} strokeWidth={1.8} /><span>连接 SMB 共享</span></button>
        </nav>
        <div class="sidebar-note"><span class="note-icon"><Folder size={19} strokeWidth={1.5} /></span><p>给每一份灵感<br />一个妥帖的位置。</p><span class="note-line"></span></div>
      </div>
      <div class="sidebar-footer"><span class="small-brand">P</span><div><strong>Pudding</strong></div><span class="version">v0.1</span></div>
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
                <button data-drop-path={crumb.path} class:drop-target={dropRegion === 'breadcrumbs' && dropTarget === crumb.path} disabled={busy} class:current={index === crumbs.length - 1} onclick={() => navigate(crumb.path)} title={crumb.path}>{info?.places.find((p) => p.path === crumb.path)?.name ?? crumb.name}</button>
              {/each}
            </nav>
            <button class="address-edit" title={`编辑路径 (${shortcutModifier}+L)`} aria-label="编辑路径" onclick={editAddress} disabled={!directory}><Pencil size={13} /></button>
          {/if}
        </div>
        <button class="icon-button refresh-button" title="刷新 (F5)" aria-label="刷新" onclick={refresh} disabled={busy || !directory}><RotateCw size={17} class={loading ? 'spin' : ''} /></button>
        <label class="search-box"><Search size={16} /><input bind:this={searchInput} bind:value={query} placeholder={searchContent ? '搜索文件名或内容' : '搜索文件名'} aria-label="搜索当前文件夹" disabled={!directory} />{#if query}<button class="clear-search" title="清除搜索" aria-label="清除搜索" onclick={() => query = ''}><X size={14} /></button>{:else}<kbd>{shortcutModifier} F</kbd>{/if}</label>
        <label class="search-content-option" title={desktop ? "同时搜索文本文件内容" : "文件内容搜索仅在桌面版可用"}><input type="checkbox" bind:checked={searchContent} disabled={!desktop || !directory} />搜索内容</label>
      </div>

      <div class="content-layout">
        <section class="browser" aria-label="文件浏览器" data-drop-path={directory?.path}>
          <div class="folder-heading">
            <div class="folder-title"><span class="eyebrow">我的空间 <span>/</span> {currentPlace?.id === 'home' ? 'HOME' : 'FILES'}</span><h1>{title}<span class="heading-dot">.</span></h1>{#if query}<p>{`“${query}” 的搜索结果`}</p>{/if}</div>
            <button class="button primary create-button" onclick={() => openDialog('create')} disabled={busy || !directory}><FolderPlus size={17} />新建文件夹</button>
          </div>

          <div class="file-toolbar">
            <div class="file-summary">{#if selected.size}<span class="selection-label">已选择 {selected.size} 项</span><button class="toolbar-text" onclick={() => selected = new Set()}>取消选择</button>{:else}<span class="folder-summary-icon"><Folder size={15} /></span><span>{folderCount} 个文件夹<span class="summary-separator">·</span>{fileCount} 个文件</span>{/if}</div>
            <div class="view-tools">
              <div class="sort-control"><ChevronsUpDown size={14} /><select aria-label="排序方式" value={sort} onchange={(event) => { sort = event.currentTarget.value as SortKey; saveFolderDisplay(); }}><option value="name">名称</option><option value="modified">修改时间</option><option value="size">大小</option><option value="type">类型</option></select><button class="sort-direction" title={descending ? '切换升序' : '切换降序'} aria-label={descending ? '切换升序' : '切换降序'} onclick={() => { descending = !descending; saveFolderDisplay(); }}>{#if descending}<ArrowDown size={13} />{:else}<ArrowUp size={13} />{/if}</button></div>
              <span class="tool-separator"></span>
              <div class="view-switch" aria-label="显示方式"><button class:active={view === 'grid'} title="图标视图" aria-label="图标视图" aria-pressed={view === 'grid'} onclick={() => setView('grid')}><LayoutGrid size={16} /></button><button class:active={view === 'list'} title="列表视图" aria-label="列表视图" aria-pressed={view === 'list'} onclick={() => setView('list')}><List size={17} /></button></div>
              <button class="icon-button compact inspector-toggle" class:active={preferences.inspector} title="信息面板" aria-label="信息面板" aria-pressed={preferences.inspector} onclick={() => preferences.inspector = !preferences.inspector}><PanelRight size={17} /></button>
              <div class="options-container"><button class="icon-button compact" title="更多显示选项" aria-label="更多显示选项" aria-expanded={optionsOpen} onclick={(event) => { event.stopPropagation(); optionsOpen = !optionsOpen; context = null; }}><MoreHorizontal size={19} /></button>
                {#if optionsOpen}<div class="dropdown-menu" role="menu"><button role="menuitemcheckbox" aria-checked={preferences.hidden} onclick={() => preferences.hidden = !preferences.hidden}>{#if preferences.hidden}<Eye size={16} />{:else}<EyeOff size={16} />{/if}<span>显示隐藏文件</span>{#if preferences.hidden}<Check size={14} />{/if}</button><button role="menuitem" onclick={() => copyPath('directory')}><Link2 size={16} /><span>复制当前路径</span></button><button role="menuitem" onclick={refresh} disabled={busy}><RotateCw size={16} /><span>刷新文件夹</span><kbd>F5</kbd></button></div>{/if}
              </div>
            </div>
          </div>

          {#if clipboard}<div class="clipboard-strip">{#if clipboard.mode === 'copy'}<Copy size={14} />{:else}<Scissors size={14} />{/if}<span>{clipboard.paths.length} 个项目等待{clipboard.mode === 'cut' ? '移动' : '复制'}</span><button onclick={paste} disabled={busy}>粘贴到此处 <ClipboardPaste size={13} /></button><button class="icon-button" title="取消剪贴操作" aria-label="取消剪贴操作" onclick={cancelClipboard} disabled={operating}><X size={13} /></button></div>{/if}

          {#if dropTarget}<div class="drop-hint" role="status">松开后选择复制或移动到「{basename(dropTarget) || dropTarget}」 · Ctrl 复制 · Shift 移动</div>{/if}

          <div class="file-area" data-drop-path={directory?.path} class:drop-target={dropRegion === 'browser' && dropTarget === directory?.path} class:loading class:list-view={view === 'list'} bind:this={fileArea} use:trackFileArea role="group" aria-label="文件和文件夹" aria-busy={busy} onpointerdown={beginMarquee} oncontextmenu={(event) => contextMenu(event)}>
            {#if initialError}
              <div class="empty-state"><span class="empty-icon error"><CircleAlert size={34} strokeWidth={1.3} /></span><h2>暂时无法打开文件夹</h2><p>{initialError}</p><button class="button" onclick={() => info ? navigate(info.home) : initialize()}><RotateCw size={15} />重新尝试</button></div>
            {:else if !directory && loading}
              <div class="empty-state"><LoaderCircle class="spin" size={28} /><p>正在读取文件夹…</p></div>
            {:else if !entries.length}
              <div class="empty-state"><span class="empty-icon">{#if query}<Search size={34} strokeWidth={1.3} />{:else}<Folder size={38} strokeWidth={1.2} />{/if}</span><h2>{query ? '没有找到匹配的项目' : '文件夹为空'}</h2><p>{query ? (searchContent ? '试试其他文件名或文本内容。' : '试试其他文件名。') : '可以创建一个文件夹。'}</p>{#if query}<button class="button" onclick={() => query = ''}>清除搜索</button>{:else}<button class="button" onclick={() => openDialog('create')} disabled={busy}><FolderPlus size={16} />新建文件夹</button>{/if}</div>
            {:else if view === 'grid'}
              <div class="file-grid" style={`--cell-size: ${preferences.size}px`}>
                {#if windowed.before}<div class="virtual-grid-spacer" style={`height: ${windowed.before}px`} aria-hidden="true"></div>{/if}
                {#each renderedEntries as entry, localIndex (entry.path)}
                  {@const index = windowed.start + localIndex}
                  <button class="file-card" draggable={!desktop && !busy} onpointerdown={(event) => beginDrag(event, entry)} ondragstart={(event) => demoDragStart(event, entry)} data-drop-path={entry.isDir ? entry.path : undefined} class:drop-target={dropRegion === 'browser' && dropTarget === entry.path} class:selected={selected.has(entry.path)} class:cut={clipboard?.mode === 'cut' && clipboard.paths.includes(entry.path)} data-index={index} aria-label={entry.name} aria-pressed={selected.has(entry.path)} title={`${entry.name}\n${entry.isDir ? '文件夹' : formatEntrySize(entry)}\n双击打开`} onclick={(event) => selectEntry(entry, event)} ondblclick={() => openEntry(entry)} oncontextmenu={(event) => contextMenu(event, entry)} disabled={operating}>
                    {#if selected.has(entry.path)}<span class="selected-check"><Check size={10} strokeWidth={3} /></span>{/if}
                    <span class="file-visual">{#if !entry.metadataPending && (isImage(entry) || isVideo(entry))}<FileThumbnail {entry} large />{:else}<FileIcon {entry} large />{/if}</span>
                    <span class="file-name">{entry.name}</span><span class="file-caption">{entry.isDir ? folderLabels[entry.name] ?? '文件夹' : formatEntrySize(entry)}</span>
                  </button>
                {/each}
                {#if windowed.after}<div class="virtual-grid-spacer" style={`height: ${windowed.after}px`} aria-hidden="true"></div>{/if}
              </div>
            {:else}
              <div class="list-header"><button onclick={() => setSort('name')}>名称{#if sort === 'name'}{#if descending}<ChevronDown size={13} />{:else}<ArrowUp size={12} />{/if}{/if}</button><button onclick={() => setSort('modified')}>修改时间</button><button onclick={() => setSort('type')}>类型</button><button onclick={() => setSort('size')}>大小</button></div>
              <div class="file-list">
                {#if windowed.before}<div style={`height: ${windowed.before}px`} aria-hidden="true"></div>{/if}
                {#each renderedEntries as entry, localIndex (entry.path)}
                  {@const index = windowed.start + localIndex}
                  <button class="file-row" draggable={!desktop && !busy} onpointerdown={(event) => beginDrag(event, entry)} ondragstart={(event) => demoDragStart(event, entry)} data-drop-path={entry.isDir ? entry.path : undefined} class:drop-target={dropRegion === 'browser' && dropTarget === entry.path} class:selected={selected.has(entry.path)} class:cut={clipboard?.mode === 'cut' && clipboard.paths.includes(entry.path)} data-index={index} aria-label={entry.name} aria-pressed={selected.has(entry.path)} onclick={(event) => selectEntry(entry, event)} ondblclick={() => openEntry(entry)} oncontextmenu={(event) => contextMenu(event, entry)} disabled={operating}>
                    <span class="row-name">{#if !entry.metadataPending && (isImage(entry) || isVideo(entry))}<FileThumbnail {entry} />{:else}<FileIcon {entry} />{/if}<span>{entry.name}</span></span><span class="row-date">{formatDate(entry.modified)}</span><span class="row-type">{fileKind(entry)}</span><span class="row-size">{entry.isDir ? '—' : formatEntrySize(entry)}</span>
                  </button>
                {/each}
                {#if windowed.after}<div style={`height: ${windowed.after}px`} aria-hidden="true"></div>{/if}
              </div>
            {/if}
          </div>

          <div class="browser-bottom"><span><span class="small-status-dot"></span>{query ? `筛选出 ${entries.length} 个项目` : `${entries.length} 个项目`}{#if selected.size}<span class="summary-separator">·</span>已选择 {selected.size} 项{#if selectedSizeKnown && selectedSize > 0}，{formatSize(selectedSize)}{/if}{/if}</span><span class="zoom-controls">{#if view === 'grid'}<LayoutGrid size={12} /><input type="range" aria-label="图标大小" min="104" max="168" step="1" value={preferences.size} oninput={(event) => preferences.size = Number(event.currentTarget.value)} /><LayoutGrid size={16} />{:else}<List size={13} /><span>详细列表</span>{/if}</span></div>
        </section>

        {#if preferences.inspector}
          <aside class="inspector" aria-label="信息面板">
            <div class="inspector-heading"><span>详细信息</span><button class="icon-button compact" title="关闭信息面板" aria-label="关闭信息面板" onclick={() => preferences.inspector = false}><X size={16} /></button></div>
            {#if focused && isImage(focused)}
              <button class="inspector-image" title="预览图片 (空格)" aria-label="预览图片" onclick={() => previewImage(focused!)} disabled={busy}><ImagePreview entry={focused} /></button>
            {:else}
              <div class="inspector-preview" class:multi={selected.size > 1}><div class="preview-orbit"></div><FileIcon entry={inspectorEntry} large />{#if selected.size > 1}<span class="multi-count">{selected.size}</span>{/if}</div>
            {/if}
            <div class="inspector-name"><h2>{selected.size > 1 ? `${selected.size} 个项目` : focused?.name ?? title}</h2><span class="type-badge">{selected.size > 1 ? '多项选择' : focused ? fileKind(focused) : '当前文件夹'}</span></div>
            <div class="inspector-divider"></div>
            <dl class="properties"><div><dt>位置</dt><dd class="path-value" title={focused?.path ?? directory?.path}>{selected.size > 1 ? directory?.path : focused?.path ?? directory?.path ?? '—'}</dd></div>{#if focused}<div><dt>类型</dt><dd>{fileKind(focused)}</dd></div>{#if !focused.isDir}<div><dt>大小</dt><dd>{formatEntrySize(focused)}</dd></div>{/if}<div><dt>修改时间</dt><dd>{formatDate(focused.modified)}</dd></div>{:else if selected.size > 1}<div><dt>已选项目</dt><dd>{selected.size} 项</dd></div><div><dt>文件大小</dt><dd>{formatSize(selectedSize)}</dd></div>{:else}<div><dt>内容</dt><dd>{directory?.entries.filter((e) => preferences.hidden || !e.hidden).length ?? 0} 个项目</dd></div><div><dt>文件夹</dt><dd>{directory?.entries.filter((e) => e.isDir && (preferences.hidden || !e.hidden)).length ?? 0}</dd></div><div><dt>文件</dt><dd>{directory?.entries.filter((e) => !e.isDir && (preferences.hidden || !e.hidden)).length ?? 0}</dd></div>{/if}</dl>
            {#if focused}<button class="button inspector-open" onclick={() => focused!.isDir ? openEntry(focused!) : openExternal(focused!)} disabled={busy}><ExternalLink size={15} />{focused.isDir ? '打开文件夹' : '使用默认应用打开'}<ArrowRight size={14} /></button>{/if}
            <div class="inspector-tip"><span class="tip-label"><Info size={14} />小提示</span><p>{selected.size ? `按住 ${shortcutModifier} 点选多个项目，按 Shift 连续选择，或拖动空白处框选。` : '单击查看文件信息，双击打开。也可以在地址栏中输入路径。'}</p></div>
          </aside>
        {/if}
      </div>

      <footer class="status-bar"><span>{#if operating}<LoaderCircle size={12} class="spin" />正在处理文件…{:else if loading}<LoaderCircle size={12} class="spin" />正在读取…{:else if searching}<LoaderCircle size={12} class="spin" />正在搜索，已扫描 {scanned} 个项目…{:else}<span class="status-dot"></span>一切就绪{/if}{#if metadataRequest && directory?.path === metadataRequest.path}<span class="skipped-label">正在读取文件信息…</span>{/if}{#if directory?.skipped}<span class="skipped-label">已跳过 {directory.skipped} 个无法读取的项目</span>{/if}</span><span>{#if !desktop}浏览器演示 · 示例数据仅保存在本次会话{:else}{info?.platform === 'macos' ? 'macOS' : info?.platform === 'windows' ? 'Windows' : 'Linux'} · 本地文件系统{/if}<span class="status-divider"></span><kbd>{shortcutModifier} L</kbd> 输入路径</span></footer>
    </main>
  </div>

  {#if notice}<div class="toast" class:error={notice.error} role={notice.error ? 'alert' : 'status'}>{#if notice.error}<CircleAlert size={18} />{:else}<Check size={18} />{/if}<span>{notice.text}</span><button class="icon-button compact" title="关闭提示" aria-label="关闭提示" onclick={() => notice = null}><X size={15} /></button></div>{/if}

  {#if marquee?.active}<div class="selection-marquee" aria-hidden="true" style={`left: ${Math.min(marquee.startX, marquee.x)}px; top: ${Math.min(marquee.startY, marquee.y)}px; width: ${Math.abs(marquee.x - marquee.startX)}px; height: ${Math.abs(marquee.y - marquee.startY)}px`}></div>{/if}

  {#if dropMenu}<div class="context-menu drop-menu" role="menu" aria-label="选择拖放操作" style={`left: ${dropMenu.x}px; top: ${dropMenu.y}px`}>
    <button role="menuitem" onclick={() => confirmDrop('copy')}><Copy size={16} /><span>复制到此处</span><kbd>Ctrl</kbd></button>
    <button role="menuitem" onclick={() => confirmDrop('cut')}><Scissors size={16} /><span>移动到此处</span><kbd>Shift</kbd></button>
    <button role="menuitem" onclick={() => dropMenu = null}><X size={16} /><span>取消</span><kbd>Esc</kbd></button>
  </div>{/if}

  {#if context}{#key context}<div class="context-menu" role="menu" style={`left: ${context.x}px; top: ${context.y}px`}>
    {#if focused && isImage(focused)}<button role="menuitem" onclick={() => previewImage(focused!)}><Eye size={16} /><span>预览图片</span><kbd>Space</kbd></button><button role="menuitem" onclick={() => { context = null; void openExternal(focused!); }}><ExternalLink size={16} /><span>使用默认应用打开</span></button>{/if}
    {#if focused && !canPreview(focused)}<button role="menuitem" onclick={() => previewImage(focused!)}><Info size={16} /><span>查看基本信息</span><kbd>Space</kbd></button>{/if}
    {#if focused && !isImage(focused) && canPreview(focused)}<button role="menuitem" onclick={() => previewImage(focused!)}><Eye size={16} /><span>预览</span><kbd>Space</kbd></button>{/if}
    {#if focused && !isImage(focused)}<button role="menuitem" onclick={() => { context = null; if (focused) void openEntry(focused); }}><ExternalLink size={16} /><span>打开</span><kbd>Enter</kbd></button>{/if}
    {#if desktop && focused && !focused.isDir}<button role="menuitem" onclick={() => void openWith(focused!)}><ExternalLink size={16} /><span>选择其他应用打开…</span></button>{/if}
    {#if focused?.isDir}<button role="menuitem" onclick={() => { const path = focused!.path; context = null; newTab(path); }}><FolderPlus size={16} /><span>在新标签页中打开</span></button><button role="menuitem" onclick={() => void openInWindow(focused!.path)}><ExternalLink size={16} /><span>在新窗口中打开</span></button>{/if}
    {#if directory && (focused?.isDir || selected.size === 0)}<button role="menuitem" onclick={() => void openTerminal(focused?.isDir ? focused.path : directory!.path)}><Terminal size={16} /><span>在终端中打开</span></button>{/if}
    {#if sugarAvailable && (selected.size || directory)}
      <div class="menu-divider"></div>
      <button role="menuitem" aria-haspopup="menu" aria-expanded={sugarMenuOpen} onclick={(event) => { event.stopPropagation(); sugarMenuOpen = !sugarMenuOpen; }} onmouseenter={() => sugarMenuOpen = true}><FileText size={16} /><span>解压缩</span><ChevronRight size={14} /></button>
      {#if sugarMenuOpen}<div class="archive-submenu" role="menu" aria-label="解压缩">
        {#if archiveSelected}<button role="menuitem" onclick={() => void runSugar('extract')}><Folder size={16} /><span>解压到当前文件夹</span></button><button role="menuitem" onclick={() => void runSugar('extract-folder')}><FolderPlus size={16} /><span>解压到同名文件夹</span></button>{/if}
        <button role="menuitem" onclick={() => void runSugar('compress')}><FileText size={16} /><span>{selected.size ? '快速压缩为 ZIP' : '快速压缩当前文件夹'}</span></button>
        <button role="menuitem" onclick={() => void runSugar('custom')}><ExternalLink size={16} /><span>使用 Sugar 自定义压缩…</span></button>
      </div>{/if}
    {/if}
    {#if selected.size}<button role="menuitem" onclick={() => copySelection('copy')}><Copy size={16} /><span>复制</span><kbd>{shortcutModifier} C</kbd></button><button role="menuitem" onclick={() => copySelection('cut')}><Scissors size={16} /><span>剪切</span><kbd>{shortcutModifier} X</kbd></button><button role="menuitem" onclick={() => copyPath('selection')}><Link2 size={16} /><span>{selected.size > 1 ? '复制项目地址' : focused?.isDir ? '复制文件夹地址' : '复制文件地址'}</span></button>{/if}
    <button role="menuitem" onclick={paste} disabled={busy || (!desktop && !clipboard)}><ClipboardPaste size={16} /><span>粘贴</span><kbd>{shortcutModifier} V</kbd></button>
    <div class="menu-divider"></div>
    <button role="menuitem" onclick={() => openDialog('create')}><FolderPlus size={16} /><span>新建文件夹</span></button>
    {#if selected.size === 1}<button role="menuitem" onclick={() => openDialog('rename')}><Pencil size={16} /><span>重命名</span><kbd>F2</kbd></button>{/if}
    {#if !selected.size}<button role="menuitem" onclick={() => copyPath('directory')}><Link2 size={16} /><span>复制当前文件夹地址</span></button>{/if}
    {#if selected.size}<div class="menu-divider"></div><button role="menuitem" class="danger-text" onclick={(event) => confirmTrash(shiftHeld || event.shiftKey)}><Trash2 size={16} /><span>{shiftHeld ? '永久删除' : '移入回收站'}</span><kbd>{shiftHeld ? 'Shift Del' : 'Del'}</kbd></button>{/if}
  </div>{/key}{/if}

  {#if transferConflict}<ConflictDialog conflicts={transferConflict.paths} onchoose={(choice) => transferConflict?.resolve(choice)} />{/if}
  {#if openWithEntry}<OpenWithDialog entry={openWithEntry} onclose={() => openWithEntry = null} />{/if}
  {#if dialog}<NameDialog mode={dialog.mode} initial={dialog.mode === 'rename' ? dialog.entry?.name : ''} busy={operating} error={dialogError} onsubmit={submitName} onclose={() => { if (!operating) dialog = null; }} />{/if}
  {#if preview}{#if isImage(preview)}<ImagePreviewDialog entries={entries.filter(isImage)} initial={preview} onclose={() => preview = null} onopen={openExternal} />{:else if canPreview(preview)}<FilePreviewDialog entry={preview} onclose={() => preview = null} onopen={openExternal} />{:else}<FileInfoDialog entry={preview} onclose={() => preview = null} />{/if}{/if}

  <dialog class="modal trash-modal" bind:this={trashDialog} oncancel={(event) => { if (operating) event.preventDefault(); }}><div class="modal-top"><span class="modal-icon danger"><Trash2 size={24} /></span><button class="icon-button" title="关闭" aria-label="关闭" onclick={() => trashDialog.close()} disabled={operating}><X size={18} /></button></div><h2>{permanentDelete ? '永久删除？' : '移入回收站？'}</h2><p>{permanentDelete ? `将永久删除选中的 ${selected.size} 个项目，此操作无法撤销。` : `将选中的 ${selected.size} 个项目移入系统回收站。你可以稍后从系统回收站中恢复。`}</p><div class="modal-actions"><button class="button" onclick={() => trashDialog.close()} disabled={operating}>取消</button><button class="button danger-button" onclick={moveToTrash} disabled={busy}>{#if operating}<LoaderCircle size={16} class="spin" />{/if}{permanentDelete ? '永久删除' : '移入回收站'}</button></div></dialog>

  <dialog class="modal smb-modal" bind:this={smbDialog} aria-labelledby="smb-title" onclose={() => { smbPassword = ''; smbShares = null; }} oncancel={(event) => { if (smbConnecting) event.preventDefault(); }}>
    <form onsubmit={connectSmb}>
      <div class="modal-top"><span class="modal-icon"><Network size={24} /></span><button type="button" class="icon-button" title="关闭连接窗口" aria-label="关闭连接窗口" disabled={smbConnecting} onclick={() => smbDialog.close()}><X size={18} /></button></div>
      <h2 id="smb-title">连接 SMB 共享</h2>
      <p>输入服务器地址及账户信息，查看并选择共享文件夹；用户名和密码均留空时以访客身份连接。</p>
      <label class="smb-address">服务器地址<input type="text" bind:value={smbAddress} oninput={() => smbShares = null} placeholder="smb://server 或 smb://server/share" autocomplete="off" spellcheck="false" required disabled={smbConnecting} /></label>
      <label class="smb-address">用户名<input type="text" bind:value={smbUsername} oninput={() => smbShares = null} autocomplete="username" disabled={smbConnecting} /></label>
      <label class="smb-address">密码<input type="password" bind:value={smbPassword} oninput={() => smbShares = null} autocomplete="off" disabled={smbConnecting} /></label>
      {#if smbShares?.length}<div class="smb-history"><strong>选择共享文件夹</strong>{#each smbShares as share}<div><button type="button" class="smb-history-address" disabled={smbConnecting} onclick={() => { const url = new URL(smbAddress.trim()); url.pathname = `/${encodeURIComponent(share)}`; smbAddress = url.toString(); smbShares = null; smbError = ''; }}><Folder size={15} /> {share}</button></div>{/each}</div>{/if}
      {#if smbHistory.length}<div class="smb-history"><strong>最近连接</strong>{#each smbHistory as address}<div><button type="button" class="smb-history-address" onclick={() => { smbAddress = address; smbShares = null; smbError = ''; }} disabled={smbConnecting}>{address}</button><button type="button" class="icon-button compact" aria-label={`删除历史 ${address}`} title="删除记录" disabled={smbConnecting} onclick={() => { smbHistory = smbHistory.filter((item) => item !== address); try { localStorage.setItem('pudding.smbHistory', JSON.stringify(smbHistory)); } catch { /* Optional persistence. */ } }}><X size={14} /></button></div>{/each}</div>{/if}
      {#if smbError}<div class="smb-error" role="alert">{smbError}</div>{/if}
      <div class="modal-actions"><button type="button" class="button" onclick={() => smbDialog.close()} disabled={smbConnecting}>取消</button><button class="button primary" type="submit" disabled={smbConnecting}>{#if smbConnecting}<LoaderCircle size={16} class="spin" />{/if}连接</button></div>
    </form>
  </dialog>
  <dialog class="modal settings-modal" bind:this={settingsDialog} aria-labelledby="settings-title">
    <div class="modal-top"><span class="modal-icon"><Settings size={24} /></span><button class="icon-button" title="关闭设置" aria-label="关闭设置" onclick={() => settingsDialog.close()}><X size={18} /></button></div>
    <h2 id="settings-title">设置</h2><p>调整文件浏览的外观与显示方式，修改会自动保存。</p>
    <div class="settings-list">
      <label class="settings-row"><span><strong>外观</strong><small>选择浅色或深色界面</small></span><select aria-label="外观" value={preferences.dark ? 'dark' : 'light'} onchange={(event) => preferences.dark = event.currentTarget.value === 'dark'}><option value="light">浅色</option><option value="dark">深色</option></select></label>
      <label class="settings-row"><span><strong>显示方式</strong><small>选择文件的默认显示方式</small></span><select aria-label="显示方式" value={preferences.view} onchange={(event) => { preferences.view = event.currentTarget.value as ViewMode; if (!directory || !Object.hasOwn(folderDisplays, directory.path)) view = preferences.view; }}><option value="grid">图标</option><option value="list">列表</option></select></label>
      <div class="settings-row"><span><strong>文件夹显示与排序记录</strong><small>清理各文件夹保存的显示方式和排序方式</small></span><button class="button" type="button" onclick={clearFolderDisplays}>清理记录</button></div>
      <label class="settings-row"><span><strong>默认终端</strong><small>选择打开文件夹时使用的终端</small></span><select aria-label="默认终端" bind:value={preferences.terminal}><option value="system">系统终端</option>{#if info?.platform === 'macos'}<option value="terminal">终端</option><option value="iterm">iTerm</option><option value="warp">Warp</option>{:else if info?.platform === 'windows'}<option value="windows-terminal">Windows Terminal</option><option value="powershell">PowerShell</option><option value="cmd">命令提示符</option>{:else if info?.platform === 'linux'}<option value="gnome-terminal">GNOME Terminal</option><option value="konsole">Konsole</option><option value="xfce4-terminal">Xfce Terminal</option><option value="xterm">XTerm</option>{/if}<option value="custom" disabled={!preferences.customTerminal}>自选应用{preferences.customTerminal ? `：${basename(preferences.customTerminal)}` : ''}</option></select></label>
      <div class="settings-row"><span><strong>终端应用</strong><small title={preferences.customTerminal}>{preferences.customTerminal || '选择本机安装的终端应用'}</small></span><button class="button" type="button" onclick={chooseTerminal}>选择应用…</button></div>
      <label class="settings-row"><span><strong>显示隐藏文件</strong><small>在文件夹中显示隐藏项目</small></span><input type="checkbox" bind:checked={preferences.hidden} /></label>
      <label class="settings-row"><span><strong>显示信息面板</strong><small>显示所选项目的详细信息</small></span><input type="checkbox" bind:checked={preferences.inspector} /></label>
      <label class="settings-row"><span><strong>图标大小</strong><small>调整图标视图中的项目大小</small></span><input type="range" min="104" max="168" step="1" bind:value={preferences.size} /></label>
    </div>
    <div class="modal-actions"><button class="button" onclick={() => settingsDialog.close()}>完成</button></div>
  </dialog>
  <dialog class="modal help-modal" bind:this={helpDialog}><div class="modal-top"><span class="modal-icon"><Keyboard size={24} /></span><button class="icon-button" title="关闭" aria-label="关闭" onclick={() => helpDialog.close()}><X size={18} /></button></div><h2>键盘快捷键</h2><p>Pudding 0.1 · Tauri + Svelte</p><dl class="shortcut-list">{#each [[`${shortcutModifier} T / W`, '新建 / 关闭标签页'], [`${shortcutModifier} L`, '输入文件夹路径'], [`${shortcutModifier} F`, '搜索文件名或内容'], ['Alt ↑ / ← / →', '上一级 / 后退 / 前进'], ['鼠标侧键', '后退 / 前进'], [`${shortcutModifier} Shift N`, '新建文件夹'], [`${shortcutModifier} A`, '选择所有项目'], [`${shortcutModifier} C / X / V`, '复制 / 剪切 / 粘贴'], ['Space', '预览或查看信息 / 关闭'], ['F2', '重命名'], ['Delete / Shift Delete', '移入回收站 / 永久删除'], ['F5', '刷新'], [`${shortcutModifier} Shift .`, '显示或隐藏隐藏文件']] as [keys, label]}<div><dt>{label}</dt><dd><kbd>{keys}</kbd></dd></div>{/each}</dl>{#if !desktop}<p class="help-demo">当前为浏览器演示。使用 <code>npm run tauri dev</code> 启动桌面版，管理真实文件。</p>{/if}</dialog>
</div>
