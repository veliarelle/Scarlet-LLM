<script lang="ts">
  import { onMount } from "svelte";
  import { Check, ChevronDown, Download, Plus, Save, Trash2, Upload } from "lucide-svelte";
  import { settings } from "$lib/stores/settings";
  import { tr } from "$lib/i18n";
  import {
    agentPresetList,
    deleteAgentPreset,
    exportAgentPreset,
    importAgentPreset,
    loadAgentPresetIntoSettings,
    overwriteAgentPreset,
    refreshAgentPresets,
    saveAgentPresetFromCurrent,
  } from "$lib/stores/agentPresets";
  import type { AgentDefinition } from "$lib/types/settings";
  import { uid } from "$lib/utils/id";
  import { clickOutside } from "$lib/utils/clickOutside";
  import Section from "./Section.svelte";
  import Row from "./Row.svelte";
  import Toggle from "./Toggle.svelte";

  let toolDefsText = $state($settings.tool_definitions ?? "[]");
  let toolDefsError = $state("");
  let agentPresetOpen = $state(false);
  let showAgentPresetInput = $state(false);
  let newAgentPresetName = $state("");
  let expandedAgentId = $state<string | null>(null);
  let agentPresetSaved = $state(false);
  let agentPresetSavedTimer: ReturnType<typeof setTimeout> | null = null;

  onMount(() => {
    refreshAgentPresets();
  });

  $effect(() => {
    toolDefsText = $settings.tool_definitions ?? "[]";
  });

  async function set(k: "web_search" | "agents" | "tools", v: boolean) {
    await settings.patch({ [k]: v } as Record<string, boolean>);
  }

  async function patchAgents(next: AgentDefinition[]) {
    await settings.patch({ agent_definitions: next });
  }

  async function addAgent() {
    const next = [
      ...($settings.agent_definitions ?? []),
      {
        id: uid(),
        name: "ResearchAgent",
        description: "",
        prompt:
          "Handle the requested subtask as a focused specialist. Use available tools when they are useful. Return a concise result for the main assistant to continue.",
        enabled: true,
        model: null,
        proxy_id: null,
        tool_names: [],
      },
    ];
    await patchAgents(next);
    expandedAgentId = next[next.length - 1].id;
  }

  async function updateAgent(id: string, patch: Partial<AgentDefinition>) {
    const next = ($settings.agent_definitions ?? []).map((agent) =>
      agent.id === id ? { ...agent, ...patch } : agent
    );
    await patchAgents(next);
  }

  async function deleteAgent(id: string) {
    await patchAgents(($settings.agent_definitions ?? []).filter((agent) => agent.id !== id));
  }

  function parseToolNames(value: string): string[] {
    return value
      .split(",")
      .map((name) => name.trim())
      .filter(Boolean);
  }

  async function saveAgentPresetCreate() {
    const name = newAgentPresetName.trim();
    if (!name) return;
    await saveAgentPresetFromCurrent(name, $settings.agent_definitions ?? []);
    newAgentPresetName = "";
    showAgentPresetInput = false;
    agentPresetOpen = false;
  }

  async function saveActiveAgentPreset() {
    if (!$settings.active_agent_preset_id) return;
    await overwriteAgentPreset($settings.active_agent_preset_id, $settings.agent_definitions ?? []);
    agentPresetSaved = true;
    if (agentPresetSavedTimer) clearTimeout(agentPresetSavedTimer);
    agentPresetSavedTimer = setTimeout(() => {
      agentPresetSaved = false;
      agentPresetSavedTimer = null;
    }, 2000);
  }

  async function loadAgentPreset(id: string) {
    await loadAgentPresetIntoSettings(id);
    expandedAgentId = null;
    agentPresetOpen = false;
  }

  async function delAgentPreset(id: string) {
    if (!confirm($tr("tools.deleteAgentPresetConfirm"))) return;
    await deleteAgentPreset(id);
  }

  async function onExportAgentPreset() {
    if (!$settings.active_agent_preset_id) return;
    await exportAgentPreset($settings.active_agent_preset_id);
  }

  async function onImportAgentPreset() {
    await importAgentPreset();
    expandedAgentId = null;
  }

  async function saveToolDefs() {
    try {
      JSON.parse(toolDefsText);
      toolDefsError = "";
      await settings.patch({ tool_definitions: toolDefsText });
    } catch {
      toolDefsError = $tr("tools.invalidJson");
    }
  }

  function insertWebSearch() {
    try {
      const arr = JSON.parse(toolDefsText || "[]") as unknown[];
      const already = arr.some(
        (t) =>
          typeof t === "object" &&
          t !== null &&
          (t as Record<string, unknown>)["type"] === "function" &&
          (t as Record<string, unknown>)["function"] !== null &&
          typeof (t as Record<string, unknown>)["function"] === "object" &&
          ((t as Record<string, unknown>)["function"] as Record<string, unknown>)["name"] === "web_search"
      );
      if (!already) {
        arr.push({
          type: "function",
          function: {
            name: "web_search",
            description: "Search the web for current information",
            parameters: {
              type: "object",
              properties: {
                query: { type: "string", description: "The search query" },
              },
              required: ["query"],
            },
          },
        });
        toolDefsText = JSON.stringify(arr, null, 2);
        saveToolDefs();
      }
    } catch {
      toolDefsError = $tr("tools.invalidJson");
    }
  }

  function insertHttpTool() {
    try {
      const arr = JSON.parse(toolDefsText || "[]") as unknown[];
      const already = arr.some(
        (t) =>
          typeof t === "object" &&
          t !== null &&
          (t as Record<string, unknown>)["type"] === "function" &&
          typeof (t as Record<string, unknown>)["function"] === "object" &&
          ((t as Record<string, unknown>)["function"] as Record<string, unknown>)["name"] ===
            "http_lookup"
      );
      if (!already) {
        arr.push({
          type: "function",
          function: {
            name: "http_lookup",
            description: "Call a configured HTTP endpoint and return its response",
            parameters: {
              type: "object",
              properties: {
                query: { type: "string", description: "Query text" },
              },
              required: ["query"],
            },
          },
          executor: {
            type: "http",
            method: "GET",
            url: "https://example.com/search?q={{query}}",
            headers: {},
            timeout_ms: 30000,
          },
        });
        toolDefsText = JSON.stringify(arr, null, 2);
        saveToolDefs();
      }
    } catch {
      toolDefsError = $tr("tools.invalidJson");
    }
  }

  const activeAgentPresetName = $derived(
    $agentPresetList.find((p) => p.id === $settings.active_agent_preset_id)?.name ??
      $tr("tools.agentPresets")
  );
</script>

<Section title={$tr("tools.capabilities")}>
  <Row label="Web search" hint={$tr("tools.webSearchHint")}>
    <Toggle value={$settings.web_search} onChange={(v) => set("web_search", v)} />
  </Row>
  <Row label="Agents" hint={$tr("tools.agentsHint")}>
    <Toggle value={$settings.agents} onChange={(v) => set("agents", v)} />
  </Row>
  {#if $settings.agents}
    <div class="inline-settings">
      <div class="preset-row">
        <div class="selector-wrap" use:clickOutside={() => (agentPresetOpen = false)}>
          <button class="selector-btn" onclick={() => (agentPresetOpen = !agentPresetOpen)} type="button">
            <span class="ellipsis">{activeAgentPresetName}</span>
            <ChevronDown size={14} color="var(--text-3)" />
          </button>
          {#if agentPresetOpen}
            <div class="dropdown">
              <div class="dropdown-group">{$tr("tools.savedAgentPresets")}</div>
              {#if $agentPresetList.length === 0}
                <div class="empty">{$tr("tools.noAgentPresets")}</div>
              {/if}
              {#each $agentPresetList as pr (pr.id)}
                <div class="dropdown-item" class:active={$settings.active_agent_preset_id === pr.id}>
                  <button class="preset-pick" onclick={() => loadAgentPreset(pr.id)}>
                    <span class="ellipsis">{pr.name}</span>
                  </button>
                  {#if $settings.active_agent_preset_id === pr.id}
                    <Check size={14} color="var(--accent)" />
                  {/if}
                  <button
                    class="preset-icon danger"
                    onclick={() => delAgentPreset(pr.id)}
                    aria-label={$tr("common.delete")}
                  >
                    <Trash2 size={13} />
                  </button>
                </div>
              {/each}
              <div class="dropdown-divider"></div>
              {#if showAgentPresetInput}
                <div class="save-row">
                  <!-- svelte-ignore a11y_autofocus -->
                  <input
                    class="text-input"
                    bind:value={newAgentPresetName}
                    onkeydown={(e) => {
                      if (e.key === "Enter") saveAgentPresetCreate();
                      if (e.key === "Escape") showAgentPresetInput = false;
                    }}
                    placeholder={$tr("tools.agentPresetNamePlaceholder")}
                    autofocus
                  />
                  <button class="save-btn" onclick={saveAgentPresetCreate} aria-label={$tr("common.create")}>
                    <Check size={16} />
                  </button>
                </div>
              {:else}
                <button class="dropdown-item add" onclick={() => (showAgentPresetInput = true)}>
                  <Plus size={14} /> {$tr("tools.createAgentPreset")}
                </button>
              {/if}
            </div>
          {/if}
        </div>

        <button
          class="preset-action"
          title={$tr("tools.exportAgentPreset")}
          onclick={onExportAgentPreset}
          disabled={!$settings.active_agent_preset_id}
          aria-label={$tr("tools.exportAgentPreset")}
        >
          <Download size={16} />
        </button>

        <button
          class="preset-action"
          title={$tr("tools.importAgentPreset")}
          onclick={onImportAgentPreset}
          aria-label={$tr("tools.importAgentPreset")}
        >
          <Upload size={16} />
        </button>

        <button
          class="preset-action"
          class:saved={agentPresetSaved}
          title={$settings.active_agent_preset_id
            ? $tr("tools.saveAgentPresetTitle")
            : $tr("tools.noActiveAgentPreset")}
          onclick={saveActiveAgentPreset}
          disabled={!$settings.active_agent_preset_id}
          aria-label={$tr("tools.saveAgentPreset")}
        >
          <Save size={16} />
        </button>
      </div>

      <div class="inline-head">
        <div class="inline-title">{$tr("tools.agentSettings")}</div>
        <button class="action-btn" onclick={addAgent}>{$tr("tools.addAgent")}</button>
      </div>
      <p class="hint">{$tr("tools.agentSettingsHint")}</p>

      {#if ($settings.agent_definitions ?? []).length === 0}
        <p class="hint">{$tr("tools.noAgents")}</p>
      {:else}
        <div class="agents-list">
          {#each $settings.agent_definitions ?? [] as agent (agent.id)}
            <div class="agent-card">
              <!-- svelte-ignore a11y_click_events_have_key_events -->
              <!-- svelte-ignore a11y_no_static_element_interactions -->
              <div
                class="agent-head"
                class:open={expandedAgentId === agent.id}
                onclick={() => (expandedAgentId = expandedAgentId === agent.id ? null : agent.id)}
              >
                <div class="agent-title">
                  <ChevronDown size={13} />
                  <span>@{agent.name || "agent"}</span>
                </div>
                <div class="agent-actions">
                  <div onclick={(e) => e.stopPropagation()}>
                    <Toggle
                      value={agent.enabled}
                      onChange={(v) => {
                        updateAgent(agent.id, { enabled: v });
                      }}
                    />
                  </div>
                  <button
                    class="danger-btn"
                    onclick={(e) => {
                      e.stopPropagation();
                      deleteAgent(agent.id);
                    }}
                  >
                    {$tr("common.delete")}
                  </button>
                </div>
              </div>

              {#if expandedAgentId === agent.id}
                <label class="field">
                  <span>{$tr("tools.agentName")}</span>
                  <input
                    value={agent.name}
                    oninput={(e) => updateAgent(agent.id, { name: (e.target as HTMLInputElement).value })}
                    placeholder="ResearchAgent"
                  />
                </label>

                <label class="field">
                  <span>{$tr("tools.agentDescription")}</span>
                  <input
                    value={agent.description}
                    oninput={(e) => updateAgent(agent.id, { description: (e.target as HTMLInputElement).value })}
                    placeholder={$tr("tools.agentDescriptionPlaceholder")}
                  />
                </label>

                <label class="field">
                  <span>{$tr("tools.agentPrompt")}</span>
                  <textarea
                    class="agent-prompt"
                    value={agent.prompt}
                    oninput={(e) => updateAgent(agent.id, { prompt: (e.target as HTMLTextAreaElement).value })}
                    rows="5"
                  ></textarea>
                </label>

                <div class="agent-grid">
                  <label class="field">
                    <span>{$tr("tools.agentModel")}</span>
                    <input
                      value={agent.model ?? ""}
                      oninput={(e) =>
                        updateAgent(agent.id, { model: (e.target as HTMLInputElement).value || null })}
                      placeholder={$settings.active_model ?? "default"}
                    />
                  </label>
                  <label class="field">
                    <span>{$tr("tools.agentProxy")}</span>
                    <input
                      value={agent.proxy_id ?? ""}
                      oninput={(e) =>
                        updateAgent(agent.id, { proxy_id: (e.target as HTMLInputElement).value || null })}
                      placeholder={$settings.active_proxy_id ?? "default"}
                    />
                  </label>
                </div>

                <label class="field">
                  <span>{$tr("tools.agentTools")}</span>
                  <input
                    value={(agent.tool_names ?? []).join(", ")}
                    oninput={(e) =>
                      updateAgent(agent.id, { tool_names: parseToolNames((e.target as HTMLInputElement).value) })}
                    placeholder="web_search, http_lookup"
                  />
                </label>
              {/if}
            </div>
          {/each}
        </div>
      {/if}
    </div>
  {/if}
  <Row label={$tr("tools.customTools")} hint={$tr("tools.customToolsHint")}>
    <Toggle value={$settings.tools} onChange={(v) => set("tools", v)} />
  </Row>
  {#if $settings.tools}
    <div class="inline-settings">
      <div class="inline-head">
        <div class="inline-title">{$tr("tools.definitions")}</div>
        <div class="defs-actions">
          <button class="action-btn" onclick={insertWebSearch}>+ web_search</button>
          <button class="action-btn" onclick={insertHttpTool}>{$tr("tools.insertHttp")}</button>
        </div>
      </div>
      <p class="hint">
        {$tr("tools.definitionsHint")}
      </p>
      <textarea
        class="defs-area"
        bind:value={toolDefsText}
        onblur={saveToolDefs}
        rows="12"
        spellcheck="false"
        placeholder="[]"
      ></textarea>
      {#if toolDefsError}
        <p class="error">{toolDefsError}</p>
      {:else}
        <p class="hint">{$tr("tools.savedOnBlur")}</p>
      {/if}
    </div>
  {/if}
</Section>

<style>
  .hint {
    font-size: 0.733rem;
    color: var(--text-3);
    line-height: 1.4;
  }
  .defs-actions {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
  }
  .preset-row {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .selector-wrap {
    position: relative;
    min-width: 0;
    flex: 1;
  }
  .selector-btn {
    width: 100%;
    height: 34px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding: 0 10px;
    background: var(--bg-3);
    border: 1px solid var(--border);
    border-radius: 8px;
    color: var(--text);
    font-size: 0.8rem;
  }
  .ellipsis {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .preset-action {
    width: 34px;
    height: 34px;
    border-radius: 8px;
    display: grid;
    place-items: center;
    color: var(--text-3);
    background: var(--bg-3);
    border: 1px solid var(--border);
  }
  .preset-action:hover:not(:disabled) {
    color: var(--text);
    background: var(--bg-4);
  }
  .preset-action.saved:not(:disabled) {
    background: color-mix(in srgb, #22c55e 78%, var(--bg-3));
    border-color: color-mix(in srgb, #22c55e 72%, var(--border));
    color: white;
  }
  .preset-action:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }
  .dropdown {
    position: absolute;
    top: calc(100% + 6px);
    left: 0;
    right: 0;
    z-index: 20;
    background: var(--bg-2);
    border: 1px solid var(--border);
    border-radius: 8px;
    box-shadow: 0 18px 40px rgba(0, 0, 0, 0.35);
    padding: 6px;
  }
  .dropdown-group {
    padding: 6px 8px;
    color: var(--text-3);
    font-size: 0.667rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.08em;
  }
  .dropdown-item {
    min-height: 30px;
    width: 100%;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 4px 7px;
    border-radius: 6px;
    color: var(--text-2);
    font-size: 0.8rem;
  }
  .dropdown-item.active,
  .dropdown-item:hover {
    background: var(--bg-3);
    color: var(--text);
  }
  .dropdown-item.add {
    justify-content: flex-start;
    color: var(--accent);
  }
  .preset-pick {
    min-width: 0;
    flex: 1;
    text-align: left;
  }
  .preset-icon {
    width: 24px;
    height: 24px;
    border-radius: 6px;
    display: grid;
    place-items: center;
    color: var(--text-3);
  }
  .preset-icon.danger:hover {
    color: var(--danger);
    background: color-mix(in srgb, var(--danger) 12%, transparent);
  }
  .dropdown-divider {
    height: 1px;
    margin: 5px 4px;
    background: var(--border);
  }
  .empty {
    padding: 8px;
    color: var(--text-3);
    font-size: 0.8rem;
  }
  .save-row {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 4px;
  }
  .text-input {
    min-width: 0;
    flex: 1;
    height: 30px;
    padding: 0 8px;
    border-radius: 7px;
    background: var(--bg-3);
    border: 1px solid var(--border);
    color: var(--text);
    font-size: 0.8rem;
  }
  .save-btn {
    width: 30px;
    height: 30px;
    display: grid;
    place-items: center;
    border-radius: 7px;
    background: var(--accent);
    color: white;
  }
  .inline-settings {
    display: flex;
    flex-direction: column;
    gap: 10px;
    margin: -4px 0 8px;
    padding: 10px 0 14px;
    border-bottom: 1px solid color-mix(in srgb, var(--border) 65%, transparent);
  }
  .inline-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
  }
  .inline-title {
    color: var(--text-2);
    font-size: 0.8rem;
    font-weight: 650;
  }
  .action-btn {
    padding: 4px 10px;
    border-radius: 6px;
    font-size: 0.8rem;
    background: var(--bg-3);
    border: 1px dashed var(--border);
    color: var(--text-3);
    transition: background 0.12s, color 0.12s;
  }
  .action-btn:hover {
    background: var(--bg-4);
    color: var(--text-2);
  }
  .defs-area {
    width: 100%;
    background: var(--bg-3);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 10px 12px;
    font-family: "JetBrains Mono", monospace;
    font-size: 0.8rem;
    color: var(--text);
    line-height: 1.5;
    resize: vertical;
    min-height: 160px;
  }
  .defs-area:focus {
    border-color: var(--accent-d);
  }
  .error {
    font-size: 0.733rem;
    color: var(--danger);
  }
  .agents-list {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .agent-card {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 12px;
    background: var(--bg-3);
    border: 1px solid var(--border);
    border-radius: 8px;
  }
  .agent-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    cursor: pointer;
  }
  .agent-title {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    color: var(--text);
    font-size: 0.867rem;
    font-weight: 650;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .agent-title :global(svg) {
    flex-shrink: 0;
    transition: transform 0.12s;
  }
  .agent-head.open .agent-title :global(svg) {
    transform: rotate(180deg);
  }
  .agent-actions {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-shrink: 0;
  }
  .danger-btn {
    padding: 4px 8px;
    border-radius: 6px;
    background: transparent;
    border: 1px solid color-mix(in srgb, var(--danger) 45%, var(--border));
    color: var(--danger);
    font-size: 0.733rem;
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: 5px;
    color: var(--text-3);
    font-size: 0.733rem;
  }
  .field input,
  .agent-prompt {
    width: 100%;
    background: var(--bg-2);
    border: 1px solid var(--border);
    border-radius: 7px;
    padding: 8px 10px;
    color: var(--text);
    font: inherit;
  }
  .agent-prompt {
    min-height: 92px;
    resize: vertical;
    line-height: 1.45;
  }
  .field input:focus,
  .agent-prompt:focus {
    border-color: var(--accent-d);
  }
  .agent-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 8px;
  }
  @media (max-width: 640px) {
    .preset-row {
      align-items: stretch;
      flex-wrap: wrap;
    }
    .selector-wrap {
      flex-basis: 100%;
    }
    .inline-head {
      align-items: flex-start;
      flex-direction: column;
    }
    .agent-grid {
      grid-template-columns: 1fr;
    }
    .agent-head {
      align-items: flex-start;
      flex-direction: column;
    }
  }
</style>
