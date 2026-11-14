export type BackgroundImage = { id: string; name: string; image: Blob };

function database(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    const request = indexedDB.open('pudding-backgrounds', 1);
    request.onupgradeneeded = () => request.result.createObjectStore('images', { keyPath: 'id' });
    request.onsuccess = () => resolve(request.result);
    request.onerror = () => reject(request.error);
  });
}

async function transaction<T>(mode: IDBTransactionMode, operation: (store: IDBObjectStore) => IDBRequest<T>): Promise<T> {
  const db = await database();
  return new Promise((resolve, reject) => {
    const tx = db.transaction('images', mode);
    const request = operation(tx.objectStore('images'));
    tx.oncomplete = () => { db.close(); resolve(request.result); };
    tx.onerror = () => { db.close(); reject(tx.error); };
    tx.onabort = () => { db.close(); reject(tx.error); };
  });
}

export const backgrounds = {
  list: () => transaction<BackgroundImage[]>('readonly', (store) => store.getAll()),
  add: (image: BackgroundImage) => transaction<IDBValidKey>('readwrite', (store) => store.put(image)),
  remove: (id: string) => transaction<undefined>('readwrite', (store) => store.delete(id)),
};
