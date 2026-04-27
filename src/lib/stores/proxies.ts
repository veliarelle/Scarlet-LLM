import { writable } from "svelte/store";
import { api } from "$lib/api/invoke";
import type { Proxy, ProxyInput } from "$lib/types/proxy";

function createProxyStore() {
  const { subscribe, set, update } = writable<Proxy[]>([]);

  return {
    subscribe,
    async load() {
      const list = await api.listProxies();
      set(list);
    },
    async create(input: ProxyInput) {
      const p = await api.createProxy(input);
      update((curr) => [...curr, p]);
      return p;
    },
    async edit(id: string, input: ProxyInput) {
      const p = await api.updateProxy(id, input);
      update((curr) => curr.map((x) => (x.id === id ? p : x)));
      return p;
    },
    async remove(id: string) {
      await api.deleteProxy(id);
      update((curr) => curr.filter((x) => x.id !== id));
    },
  };
}

export const proxies = createProxyStore();
