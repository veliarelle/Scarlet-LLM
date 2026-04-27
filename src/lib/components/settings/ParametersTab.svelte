<script lang="ts">
  import { Plus, Trash2 } from "lucide-svelte";
  import { tr } from "$lib/i18n";
  import { settings } from "$lib/stores/settings";
  import { uid } from "$lib/utils/id";
  import type { ParamEntry } from "$lib/types/settings";
  import Section from "./Section.svelte";
  import Toggle from "./Toggle.svelte";

  async function setParams(next: ParamEntry[]) {
    await settings.patch({ params: next });
  }

  async function addParam() {
    await setParams([
      ...$settings.params,
      { id: uid(), key: "", value: "", enabled: true },
    ]);
  }

  async function addFromJson() {
    const json = prompt($tr("params.jsonPrompt", { example: '{"my_param": 0.5, "stop": ["<END>"]}' }));
    if (!json) return;
    try {
      const obj = JSON.parse(json);
      if (obj === null || typeof obj !== "object" || Array.isArray(obj)) {
        alert($tr("params.objectExpected"));
        return;
      }
      const entries: ParamEntry[] = Object.entries(obj).map(([k, v]) => ({
        id: uid(),
        key: k,
        value: typeof v === "string" ? v : JSON.stringify(v),
        enabled: true,
      }));
      await setParams([...$settings.params, ...entries]);
    } catch {
      alert($tr("params.invalidJson"));
    }
  }

  async function update(id: string, patch: Partial<ParamEntry>) {
    await setParams($settings.params.map((p) => (p.id === id ? { ...p, ...patch } : p)));
  }

  async function remove(id: string) {
    await setParams($settings.params.filter((p) => p.id !== id));
  }
</script>

<Section title={$tr("params.section")}>
  <div class="params">
    {#each $settings.params as p (p.id)}
      <div class="row">
        <Toggle value={p.enabled} onChange={(v) => update(p.id, { enabled: v })} />
        <input
          class="text-input key"
          value={p.key}
          oninput={(e) => update(p.id, { key: (e.target as HTMLInputElement).value })}
          placeholder="key"
        />
        <input
          class="text-input val"
          value={p.value}
          oninput={(e) => update(p.id, { value: (e.target as HTMLInputElement).value })}
          placeholder="value"
        />
        <button class="del-btn" onclick={() => remove(p.id)} aria-label={$tr("params.delete")}>
          <Trash2 size={14} color="var(--text-3)" />
        </button>
      </div>
    {/each}
    <div class="actions">
      <button class="add-btn" onclick={addParam}>
        <Plus size={14} /> {$tr("params.add")}
      </button>
      <button class="add-btn" onclick={addFromJson}>
        <Plus size={14} /> {$tr("params.addFromJson")}
      </button>
    </div>
  </div>
</Section>

<style>
  .params {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 7px;
  }
  .text-input {
    background: var(--bg-3);
    border: 1px solid var(--border);
    border-radius: 7px;
    padding: 6px 10px;
    color: var(--text);
    font-size: 13px;
  }
  .text-input:focus {
    border-color: var(--accent-d);
  }
  .text-input.key {
    width: 130px;
    flex-shrink: 0;
    font-family: "JetBrains Mono", monospace;
  }
  .text-input.val {
    flex: 1;
    min-width: 0;
  }
  .del-btn {
    display: flex;
    align-items: center;
    opacity: 0.5;
  }
  .del-btn:hover {
    opacity: 1;
  }
  .actions {
    display: flex;
    gap: 8px;
    margin-top: 4px;
  }
  .add-btn {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px;
    border-radius: 7px;
    font-size: 12px;
    background: var(--bg-3);
    border: 1px dashed var(--border);
    color: var(--text-3);
    transition: background 0.12s, color 0.12s;
  }
  .add-btn:hover {
    background: var(--bg-4);
    color: var(--text-2);
  }
</style>
