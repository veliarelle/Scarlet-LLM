import { writable } from "svelte/store";
import { api } from "$lib/api/invoke";
import { DEFAULT_SETTINGS, type Settings } from "$lib/types/settings";

function createSettingsStore() {
  const { subscribe, set, update } = writable<Settings>(DEFAULT_SETTINGS);

  return {
    subscribe,
    async load() {
      const s = await api.getSettings();
      set(s);
    },
    async patch(patch: Partial<Settings>) {
      let next!: Settings;
      update((curr) => {
        next = { ...curr, ...patch };
        return next;
      });
      await api.saveSettings(next);
    },
  };
}

export const settings = createSettingsStore();
