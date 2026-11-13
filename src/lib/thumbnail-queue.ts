// Full images can be several megabytes. Keep scrolling a remote photo folder
// from starting dozens of simultaneous reads, and drop requests that scrolled away.
const limit = 2;
let running = 0;
const waiting: Array<() => void> = [];

export function queueThumbnail<T>(load: () => Promise<T>, active: () => boolean): Promise<T | null> {
  return new Promise((resolve, reject) => {
    const start = () => {
      if (!active()) { resolve(null); next(); return; }
      running++;
      Promise.resolve().then(load).then(resolve, reject).finally(() => { running--; next(); });
    };
    waiting.push(start);
    next();
  });
}

function next() {
  while (running < limit && waiting.length) waiting.shift()!();
}
