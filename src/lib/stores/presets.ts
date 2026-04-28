import { get, writable } from "svelte/store";
import { api } from "$lib/api/invoke";
import type { PresetMeta } from "$lib/types/preset";
import type { PresetUtilities, Prompt, PromptUtilities } from "$lib/types/settings";
import { settings } from "./settings";

export const presetList = writable<PresetMeta[]>([]);

export async function refreshPresets() {
  const list = await api.listPresets();
  presetList.set(list);
}

export async function loadPresetIntoSettings(id: string) {
  const p = await api.loadPreset(id);
  const current = get(settings);
  await settings.patch({
    prompts: p.prompts,
    utilities: {
      ...current.utilities,
      summarize_prompt_id: p.utilities?.summarize_prompt_id ?? null,
    },
    active_preset_id: id,
  });
}

function presetUtilitiesFromSettings(utilities: PromptUtilities): PresetUtilities {
  return { summarize_prompt_id: utilities.summarize_prompt_id ?? null };
}

export async function savePresetFromCurrent(name: string, prompts: Prompt[], utilities: PromptUtilities) {
  const p = await api.createPreset(name, prompts, presetUtilitiesFromSettings(utilities));
  await settings.patch({ active_preset_id: p.id });
  await refreshPresets();
  return p;
}

export async function deletePreset(id: string) {
  await api.deletePreset(id);
  await refreshPresets();
}

export async function exportPreset(id: string) {
  return api.exportPreset(id);
}

export async function importPreset() {
  const preset = await api.importPreset();
  await refreshPresets();
  if (preset) await loadPresetIntoSettings(preset.id);
  return preset;
}

export async function overwritePreset(id: string, prompts: Prompt[], utilities: PromptUtilities) {
  const p = await api.loadPreset(id);
  p.prompts = prompts;
  p.utilities = presetUtilitiesFromSettings(utilities);
  await api.savePreset(p);
  await refreshPresets();
}
