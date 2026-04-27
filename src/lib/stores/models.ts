import { writable } from "svelte/store";
import { api } from "$lib/api/invoke";
import type { Model } from "$lib/types/chat";

interface ModelsState {
  loading: boolean;
  error: string | null;
  list: Model[];
  proxyId: string | null;
}

const initial: ModelsState = { loading: false, error: null, list: [], proxyId: null };

function createModelsStore() {
  const { subscribe, update, set } = writable<ModelsState>(initial);

  return {
    subscribe,
    async load(proxyId: string) {
      update((s) => ({ ...s, loading: true, error: null, proxyId }));
      try {
        const list = await api.listModels(proxyId);
        update((s) => (s.proxyId === proxyId ? { ...s, loading: false, list } : s));
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e);
        update((s) => (s.proxyId === proxyId ? { ...s, loading: false, error: msg } : s));
      }
    },
    reset() {
      set(initial);
    },
  };
}

export const models = createModelsStore();
