export const shortcutDefaults = {
  newTab: ['新建标签页', 'Mod+KeyT'], closeTab: ['关闭标签页', 'Mod+KeyW'], address: ['输入文件夹路径', 'Mod+KeyL'], search: ['搜索', 'Mod+KeyF'],
  back: ['后退', 'Alt+ArrowLeft'], forward: ['前进', 'Alt+ArrowRight'], up: ['上一级', 'Alt+ArrowUp'], refresh: ['刷新', 'F5'],
  create: ['新建文件夹', 'Mod+Shift+KeyN'], hidden: ['显示或隐藏隐藏文件', 'Mod+Shift+Period'], selectAll: ['全选', 'Mod+KeyA'], copy: ['复制', 'Mod+KeyC'], cut: ['剪切', 'Mod+KeyX'], paste: ['粘贴', 'Mod+KeyV'], rename: ['重命名', 'F2'], trash: ['移入回收站', 'Delete'], permanentDelete: ['永久删除', 'Shift+Delete'], preview: ['预览或关闭预览', 'Space']
} as const;
export type ShortcutAction = keyof typeof shortcutDefaults;
export type Shortcuts = Record<ShortcutAction, string>;
export const defaultShortcuts = (): Shortcuts => Object.fromEntries(Object.entries(shortcutDefaults).map(([id, [, key]]) => [id, key])) as Shortcuts;
export function loadShortcuts(): Shortcuts {
  const defaults = defaultShortcuts();
  try {
    const saved = JSON.parse(localStorage.getItem('pudding.shortcuts') || '{}');
    const result = { ...defaults };
    for (const id of Object.keys(defaults) as ShortcutAction[]) if (typeof saved[id] === 'string' && /^(?:(?:Mod|Ctrl|Meta|Alt|Shift)\+)*(?:Key[A-Z]|Digit[0-9]|Arrow(?:Left|Right|Up|Down)|F(?:[1-9]|1[0-2])|Period|Comma|Space|Delete|Backspace|Enter|Home|End|PageUp|PageDown)$/.test(saved[id])) result[id] = saved[id];
    return new Set(Object.values(result)).size === Object.keys(result).length ? result : defaults;
  } catch { return defaults; }
}
export function shortcutFromEvent(event: KeyboardEvent): string | null {
  const code = event.code || event.key;
  if (!/^(?:Key[A-Z]|Digit[0-9]|Arrow(?:Left|Right|Up|Down)|F(?:[1-9]|1[0-2])|Period|Comma|Space|Delete|Backspace|Enter|Home|End|PageUp|PageDown)$/.test(code)) return null;
  const mac = /Mac|iPhone|iPad/.test(navigator.platform);
  const parts = [mac ? event.metaKey && 'Mod' : event.ctrlKey && 'Mod', mac ? event.ctrlKey && 'Ctrl' : event.metaKey && 'Meta', event.altKey && 'Alt', event.shiftKey && 'Shift', code].filter(Boolean);
  return parts.join('+');
}
export function formatShortcut(value: string): string {
  const mac = /Mac|iPhone|iPad/.test(navigator.platform);
  return value.split('+').map(part => ({ Mod: mac ? '⌘' : 'Ctrl', Meta: '⌘', Alt: 'Alt', Shift: 'Shift', Period: '.', Comma: ',', Space: 'Space', ArrowLeft: '←', ArrowRight: '→', ArrowUp: '↑', ArrowDown: '↓' } as Record<string, string>)[part] || part.replace(/^Key|^Digit/, '')).join(' + ');
}
