<script lang="ts">
  import { settings } from "$lib/stores/settings";
  import Section from "./Section.svelte";
  import Row from "./Row.svelte";
  import Toggle from "./Toggle.svelte";

  let toolDefsText = $state($settings.tool_definitions ?? "[]");
  let toolDefsError = $state("");

  $effect(() => {
    toolDefsText = $settings.tool_definitions ?? "[]";
  });

  async function set(k: "web_search" | "agents" | "tools", v: boolean) {
    await settings.patch({ [k]: v } as Record<string, boolean>);
  }

  async function saveToolDefs() {
    try {
      JSON.parse(toolDefsText);
      toolDefsError = "";
      await settings.patch({ tool_definitions: toolDefsText });
    } catch {
      toolDefsError = "Невалидный JSON";
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
      toolDefsError = "Невалидный JSON";
    }
  }
</script>

<Section title="Capabilities">
  <Row label="Web search" hint="Встроенный поиск (Anthropic / OpenAI)">
    <Toggle value={$settings.web_search} onChange={(v) => set("web_search", v)} />
  </Row>
  <Row label="Agents" hint="Пока не работает — будет реализовано позже">
    <Toggle value={$settings.agents} onChange={(v) => set("agents", v)} />
  </Row>
  <Row label="Custom tools" hint="Передаёт tool definitions в запрос">
    <Toggle value={$settings.tools} onChange={(v) => set("tools", v)} />
  </Row>
</Section>

{#if $settings.tools}
  <Section title="Tool definitions">
    <p class="hint">
      JSON-массив tool definitions в формате OpenAI function calling. Передаётся в каждый запрос при включённых Custom tools.
    </p>
    <div class="defs-actions">
      <button class="action-btn" onclick={insertWebSearch}>+ web_search</button>
    </div>
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
      <p class="hint">Изменения сохраняются при потере фокуса.</p>
    {/if}
  </Section>
{/if}

<style>
  .hint {
    font-size: 11px;
    color: var(--text-3);
    line-height: 1.4;
  }
  .defs-actions {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
  }
  .action-btn {
    padding: 4px 10px;
    border-radius: 6px;
    font-size: 12px;
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
    font-size: 12px;
    color: var(--text);
    line-height: 1.5;
    resize: vertical;
    min-height: 160px;
  }
  .defs-area:focus {
    border-color: var(--accent-d);
  }
  .error {
    font-size: 11px;
    color: var(--danger);
  }
</style>
