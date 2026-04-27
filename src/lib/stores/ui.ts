import { writable } from "svelte/store";

const isMobile = typeof window !== "undefined" && window.innerWidth < 768;

export const sidebarOpen = writable<boolean>(!isMobile);
export const settingsOpen = writable<boolean>(false);
export const proxyPanelOpen = writable<boolean>(false);
export const incognito = writable<boolean>(false);
export const selectMode = writable<boolean>(false);
export const imageMode = writable<boolean>(false);
export const activeGenerationId = writable<string | null>(null);
