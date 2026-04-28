import { get, writable } from "svelte/store";
import { api } from "$lib/api/invoke";
import type { AgentDefinition, AgentPresetMeta } from "$lib/types/settings";
import { settings } from "./settings";

export const agentPresetList = writable<AgentPresetMeta[]>([]);

function cloneAgents(agents: AgentDefinition[]): AgentDefinition[] {
  return agents.map((agent) => ({
    ...agent,
    tool_names: [...(agent.tool_names ?? [])],
  }));
}

export async function refreshAgentPresets() {
  const list = await api.listAgentPresets();
  agentPresetList.set(list);
}

export async function loadAgentPresetIntoSettings(id: string) {
  const p = await api.loadAgentPreset(id);
  await settings.patch({
    agent_definitions: cloneAgents(p.agents),
    active_agent_preset_id: id,
  });
}

export async function saveAgentPresetFromCurrent(name: string, agents: AgentDefinition[]) {
  const p = await api.createAgentPreset(name, cloneAgents(agents));
  await settings.patch({ active_agent_preset_id: p.id });
  await refreshAgentPresets();
  return p;
}

export async function overwriteAgentPreset(id: string, agents: AgentDefinition[]) {
  const p = await api.loadAgentPreset(id);
  p.agents = cloneAgents(agents);
  await api.saveAgentPreset(p);
  await refreshAgentPresets();
}

export async function deleteAgentPreset(id: string) {
  await api.deleteAgentPreset(id);
  const current = get(settings);
  if (current.active_agent_preset_id === id) {
    await settings.patch({ active_agent_preset_id: null });
  }
  await refreshAgentPresets();
}

export async function exportAgentPreset(id: string) {
  return api.exportAgentPreset(id);
}

export async function importAgentPreset() {
  const preset = await api.importAgentPreset();
  await refreshAgentPresets();
  if (preset) await loadAgentPresetIntoSettings(preset.id);
  return preset;
}
