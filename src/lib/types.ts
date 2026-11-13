export interface FileEntry {
  name: string;
  path: string;
  isDir: boolean;
  isSymlink: boolean;
  size: number;
  modified: number | null;
  hidden: boolean;
  extension: string;
  metadataPending?: boolean;
  metadataUnavailable?: boolean;
}

export interface DirectoryListing {
  path: string;
  parent: string | null;
  entries: FileEntry[];
  skipped: number;
}

export interface Place {
  id: string;
  name: string;
  path: string;
}

export interface Volume extends Place {
  ejectable: boolean;
}

export interface AppInfo {
  home: string;
  platform: string;
  places: Place[];
  roots: Volume[];
}

export type SortKey = 'name' | 'modified' | 'size' | 'type';
export type ViewMode = 'grid' | 'list';
export type Clipboard = { paths: string[]; mode: 'copy' | 'cut' } | null;
