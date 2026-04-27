// Svelte action: вызывает callback при mousedown за пределами элемента.
export function clickOutside(node: HTMLElement, cb: () => void) {
  function handle(e: MouseEvent) {
    if (!node.contains(e.target as Node)) cb();
  }
  document.addEventListener("mousedown", handle);
  return {
    destroy() {
      document.removeEventListener("mousedown", handle);
    },
    update(newCb: () => void) {
      cb = newCb;
    },
  };
}
