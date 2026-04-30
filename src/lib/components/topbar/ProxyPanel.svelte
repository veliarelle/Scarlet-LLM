<script lang="ts">
  import { Server, X, Plus, Pencil, Trash2, Save } from "lucide-svelte";
  import { onMount } from "svelte";
  import { tr } from "$lib/i18n";
  import { proxies } from "$lib/stores/proxies";
  import { settings } from "$lib/stores/settings";
  import { models } from "$lib/stores/models";
  import { proxyPanelOpen } from "$lib/stores/ui";
  import { clickOutside } from "$lib/utils/clickOutside";
  import {
    PROXY_KIND_LABELS,
    type Proxy,
    type ProxyKind,
  } from "$lib/types/proxy";

  const BASE_URL_HINTS: Record<ProxyKind, string> = {
    openai_compat: "https://api.openai.com/v1",
    anthropic_native: "https://api.anthropic.com",
    google_native: "https://generativelanguage.googleapis.com",
    openai_responses: "https://api.openai.com/v1",
    text_completions: "http://localhost:5000/v1",
    grok: "https://api.x.ai/v1",
    glm: "https://api.z.ai/api/paas/v4",
    deepseek: "https://api.deepseek.com",
    mistral: "https://api.mistral.ai/v1",
    moonshot: "https://api.moonshot.ai/v1",
    openrouter: "https://openrouter.ai/api/v1",
  };

  let expandedId = $state<string | null>(null);
  let panelEl = $state<HTMLDivElement | null>(null);
  let panelShift = $state(0);

  // Локальные edit-буферы по id, чтобы не дёргать backend на каждое нажатие
  let buffers = $state<Record<string, { name: string; base_url: string; key: string; kind: ProxyKind }>>({});
  let savedMap = $state<Record<string, boolean>>({});
  let savedTimerMap = new Map<string, ReturnType<typeof setTimeout>>();

  function placePanel() {
    if (!panelEl) return;
    const margin = 8;
    const scale = Number(getComputedStyle(document.documentElement).getPropertyValue("--ui-scale")) || 1;
    panelShift = 0;
    requestAnimationFrame(() => {
      if (!panelEl) return;
      const rect = panelEl.getBoundingClientRect();
      if (rect.left < margin) {
        panelShift = (margin - rect.left) / scale;
      } else if (rect.right > window.innerWidth - margin) {
        panelShift = (window.innerWidth - margin - rect.right) / scale;
      }
    });
  }

  onMount(() => {
    placePanel();
    const update = () => placePanel();
    window.addEventListener("resize", update);
    window.addEventListener("scroll", update, true);
    return () => {
      window.removeEventListener("resize", update);
      window.removeEventListener("scroll", update, true);
    };
  });

  function close() {
    proxyPanelOpen.set(false);
    expandedId = null;
  }

  async function activate(p: Proxy) {
    await settings.patch({ active_proxy_id: p.id, active_model: null });
    await models.load(p.id);
  }

  function startEdit(p: Proxy) {
    expandedId = expandedId === p.id ? null : p.id;
    if (expandedId === p.id) {
      buffers[p.id] = { name: p.name, base_url: p.base_url, key: "", kind: p.kind };
    }
  }

  async function saveEdit(p: Proxy) {
    const buf = buffers[p.id];
    if (!buf) return;
    await proxies.edit(p.id, buf);
    savedMap = { ...savedMap, [p.id]: true };
    const oldTimer = savedTimerMap.get(p.id);
    if (oldTimer) clearTimeout(oldTimer);
    const timer = setTimeout(() => {
      savedMap = { ...savedMap, [p.id]: false };
      savedTimerMap.delete(p.id);
    }, 2000);
    savedTimerMap.set(p.id, timer);
  }

  async function remove(p: Proxy) {
    if (!confirm($tr("proxy.deleteConfirm", { name: p.name }))) return;
    await proxies.remove(p.id);
    if ($settings.active_proxy_id === p.id) {
      await settings.patch({ active_proxy_id: null, active_model: null });
      models.reset();
    }
  }

  async function add() {
    const p = await proxies.create({
      name: $tr("proxy.new"),
      base_url: "",
      key: "",
      kind: "openai_compat",
    });
    expandedId = p.id;
    buffers[p.id] = { name: p.name, base_url: p.base_url, key: "", kind: p.kind };
  }

  function displayBaseUrl(p: Proxy): string {
    return p.base_url || BASE_URL_HINTS[p.kind];
  }
</script>

<div
  bind:this={panelEl}
  class="panel"
  style={`--panel-shift: ${panelShift}px;`}
  use:clickOutside={close}
>
  <div class="panel-hdr">
    <Server size={15} color="var(--accent)" />
    <span>{$tr("proxy.panelTitle")}</span>
    <button class="hdr-close" onclick={close} aria-label={$tr("common.close")}>
      <X size={16} />
    </button>
  </div>

  <div class="proxy-list">
    {#if $proxies.length === 0}
      <div class="empty">{$tr("proxy.empty")}</div>
    {/if}
    {#each $proxies as p (p.id)}
      <div class="entry" class:active={$settings.active_proxy_id === p.id}>
        <div class="entry-hdr">
          <button class="activate-btn" onclick={() => activate(p)}>
            <div class="dot" class:active={$settings.active_proxy_id === p.id}></div>
            <div class="info">
              <span class="name">{p.name}</span>
              <span class="url">{displayBaseUrl(p)}</span>
            </div>
          </button>
          <button
            class="icon-btn"
            title={$tr("common.edit")}
            onclick={() => startEdit(p)}
            aria-label={$tr("common.edit")}
          >
            <Pencil size={14} />
          </button>
          <button
            class="icon-btn"
            title={$tr("common.delete")}
            onclick={() => remove(p)}
            aria-label={$tr("common.delete")}
          >
            <Trash2 size={14} />
          </button>
        </div>

        {#if expandedId === p.id && buffers[p.id]}
          <div class="entry-edit">
            <input
              class="text-input"
              bind:value={buffers[p.id].name}
              placeholder={$tr("proxy.namePlaceholder")}
            />
            <button
              class="save-btn"
              class:saved={savedMap[p.id]}
              onclick={() => saveEdit(p)}
              title={$tr("common.save")}
              aria-label={$tr("common.save")}
            >
              <Save size={14} />
            </button>
            <input
              class="text-input"
              bind:value={buffers[p.id].base_url}
              placeholder={BASE_URL_HINTS[buffers[p.id].kind]}
            />
            <input
              class="text-input"
              type="password"
              bind:value={buffers[p.id].key}
              placeholder={p.has_key ? $tr("proxy.apiKeySaved") : $tr("proxy.apiKey")}
            />
            <select
              class="text-input"
              bind:value={buffers[p.id].kind}
            >
              {#each Object.entries(PROXY_KIND_LABELS) as [v, label] (v)}
                <option value={v}>{label}</option>
              {/each}
            </select>
          </div>
        {/if}
      </div>
    {/each}
  </div>

  <button class="add-btn" onclick={add}>
    <Plus size={15} /> {$tr("proxy.add")}
  </button>
</div>

<style>
  .panel {
    position: absolute;
    top: calc(100% + 6px);
    left: 50%;
    transform: translateX(calc(-50% + var(--panel-shift, 0px)));
    width: min(380px, calc((100vw / var(--app-scale, 1)) - 16px));
    max-height: min(70vh, calc((100vh / var(--app-scale, 1)) - 70px));
    z-index: 200;
    background: var(--bg-2);
    border: 1px solid var(--border);
    border-radius: 14px;
    box-shadow: var(--shadow);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .panel-hdr {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 12px 14px;
    border-bottom: 1px solid var(--border);
    font-size: 13px;
    font-weight: 600;
  }
  .hdr-close {
    margin-left: auto;
    color: var(--text-3);
    display: flex;
  }
  .hdr-close:hover {
    color: var(--text);
  }

  .proxy-list {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 6px;
  }
  .empty {
    color: var(--text-3);
    font-size: 13px;
    padding: 12px;
    text-align: center;
  }
  .entry {
    border-radius: 8px;
    overflow: hidden;
    margin-bottom: 4px;
    border: 1px solid var(--border);
  }
  .entry.active {
    border-color: var(--accent-d);
  }
  .entry-hdr {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 9px 10px;
    transition: background 0.1s;
  }
  .entry-hdr:hover {
    background: var(--bg-3);
  }
  .activate-btn {
    display: flex;
    align-items: center;
    gap: 8px;
    flex: 1;
    min-width: 0;
    text-align: left;
    color: inherit;
    cursor: pointer;
  }
  .info {
    flex: 1;
    min-width: 0;
  }
  .name {
    display: block;
    font-size: 13px;
    font-weight: 500;
  }
  .url {
    display: block;
    font-size: 11px;
    color: var(--text-3);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .icon-btn {
    display: flex;
    color: var(--text-3);
    padding: 4px;
    border-radius: 5px;
  }
  .icon-btn:hover {
    background: var(--bg-4);
    color: var(--text-2);
  }

  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--border);
    flex-shrink: 0;
  }
  .dot.active {
    background: oklch(62% 0.18 145);
  }

  .entry-edit {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 6px;
    padding: 8px 10px;
    background: var(--bg-3);
  }
  .text-input {
    background: var(--bg-3);
    border: 1px solid var(--border);
    border-radius: 7px;
    padding: 6px 10px;
    width: 100%;
    color: var(--text);
    font-size: 13px;
  }
  .entry-edit .text-input {
    background: var(--bg-2);
  }
  .entry-edit .text-input:not(:first-child),
  .entry-edit select {
    grid-column: 1 / -1;
  }
  .text-input:focus {
    border-color: var(--accent-d);
  }
  .text-input option {
    background: var(--bg-2);
    color: var(--text);
  }
  .save-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 34px;
    min-height: 34px;
    padding: 0;
    border-radius: 7px;
    background: var(--accent-d);
    border: 1px solid color-mix(in srgb, var(--accent-d) 72%, white);
    color: white;
    font-size: 12px;
    font-weight: 600;
    transition: background 0.12s, border-color 0.12s;
  }
  .save-btn:hover {
    background: color-mix(in srgb, var(--accent-d) 82%, white);
  }
  .save-btn.saved {
    background: color-mix(in srgb, #22c55e 78%, var(--bg-3));
    border-color: color-mix(in srgb, #22c55e 70%, var(--border));
  }

  .add-btn {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 10px 14px;
    color: var(--text-3);
    font-size: 13px;
    border-top: 1px solid var(--border);
    transition: background 0.1s, color 0.1s;
  }
  .add-btn:hover {
    background: var(--bg-3);
    color: var(--text-2);
  }
</style>
