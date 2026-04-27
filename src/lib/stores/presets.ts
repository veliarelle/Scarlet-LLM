import { writable } from "svelte/store";
import { api } from "$lib/api/invoke";
import type { PresetMeta } from "$lib/types/preset";
import type { Prompt } from "$lib/types/settings";
import { settings } from "./settings";

export const presetList = writable<PresetMeta[]>([]);

export async function refreshPresets() {
  const list = await api.listPresets();
  presetList.set(list);
}

export async function loadPresetIntoSettings(id: string) {
  const p = await api.loadPreset(id);
  await settings.patch({ prompts: p.prompts, active_preset_id: id });
}

export async function savePresetFromCurrent(name: string, prompts: Prompt[]) {
  const p = await api.createPreset(name, prompts);
  await settings.patch({ active_preset_id: p.id });
  await refreshPresets();
  return p;
}

export async function deletePreset(id: string) {
  await api.deletePreset(id);
  await refreshPresets();
}

export async function overwritePreset(id: string, prompts: Prompt[]) {
  const p = await api.loadPreset(id);
  p.prompts = prompts;
  await api.savePreset(p);
  await refreshPresets();
}
