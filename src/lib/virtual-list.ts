export interface VirtualWindow {
  start: number;
  end: number;
  before: number;
  after: number;
  columns: number;
  rowStep: number;
  top: number;
}

/** Keep DOM work bounded while retaining the full ordered list for selection and search. */
export function virtualWindow(count: number, view: 'grid' | 'list', cellSize: number, width: number, height: number, scrollTop: number, topPadding: number): VirtualWindow {
  const columns = view === 'grid' ? Math.max(1, Math.floor((width + 10) / (cellSize + 10))) : 1;
  const gap = view === 'grid' ? 16 : 0;
  const rowStep = view === 'grid' ? cellSize + 17 + gap : 43;
  const top = view === 'grid' ? topPadding : 42;
  if (count <= 300 || width <= 0 || height <= 0) return { start: 0, end: count, before: 0, after: 0, columns, rowStep, top };
  const rows = Math.ceil(count / columns);
  const first = Math.min(rows - 1, Math.max(0, Math.floor((scrollTop - top) / rowStep) - 3));
  const last = Math.min(rows, Math.max(first + 1, Math.ceil((scrollTop + height - top) / rowStep) + 3));
  return { start: first * columns, end: Math.min(count, last * columns), before: first ? first * rowStep - gap : 0, after: last < rows ? (rows - last) * rowStep - gap : 0, columns, rowStep, top };
}
