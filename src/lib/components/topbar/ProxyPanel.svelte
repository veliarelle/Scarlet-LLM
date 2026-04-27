<script lang="ts">
  import { Server, X, Plus, Pencil, Trash2, Eye, EyeOff } from "lucide-svelte";
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
  };

  let expandedId = $state<string | null>(null);
  let showPwdMap = $state<Record<string, boolean>>({});

  // Локальные edit-буферы по id, чтобы не дёргать backend на каждое нажатие
  let buffers = $state<Record<string, { name: string; base_url: string; key: string; kind: ProxyKind }>>({});

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
      buffers[p.id] = { name: p.name, base_url: p.base_url, key: p.key, kind: p.kind };
    }
  }

  async function saveEdit(p: Proxy) {
    const buf = buffers[p.id];
    if (!buf) return;
    await proxies.edit(p.id, buf);
  }

  async function remove(p: Proxy) {
    if (!confirm(`Удалить «${p.name}»?`)) return;
    await proxies.remove(p.id);
    if ($settings.active_proxy_id === p.id) {
      await settings.patch({ active_proxy_id: null, active_model: null });
      models.reset();
    }
  }

  async function add() {
    const p = await proxies.create({
      name: "Новая прокси",
      base_url: "",
      key: "",
      kind: "openai_compat",
    });
    expandedId = p.id;
    buffers[p.id] = { name: p.name, base_url: p.base_url, key: p.key, kind: p.kind };
  }
</script>

<div class="panel" use:clickOutside={close}>
  <div class="panel-hdr">
    <Server size={15} color="var(--accent)" />
    <span>Прокси</span>
    <button class="hdr-close" onclick={close} aria-label="Закрыть">
      <X size={16} />
    </button>
  </div>

  <div class="proxy-list">
    {#if $proxies.length === 0}
      <div class="empty">Нет прокси</div>
    {/if}
    {#each $proxies as p (p.id)}
      <div class="entry" class:active={$settings.active_proxy_id === p.id}>
        <div class="entry-hdr">
          <button class="activate-btn" onclick={() => activate(p)}>
            <div class="dot" class:active={$settings.active_proxy_id === p.id}></div>
            <div class="info">
              <span class="name">{p.name}</span>
              <span class="url">{p.base_url || "Нет URL"}</span>
            </div>
          </button>
          <button
            class="icon-btn"
            title="Редактировать"
            onclick={() => startEdit(p)}
            aria-label="Редактировать"
          >
            <Pencil size={14} />
          </button>
          <button
            class="icon-btn"
            title="Удалить"
            onclick={() => remove(p)}
            aria-label="Удалить"
          >
            <Trash2 size={14} />
          </button>
        </div>

        {#if expandedId === p.id && buffers[p.id]}
          <div class="entry-edit">
            <input
              class="text-input"
              bind:value={buffers[p.id].name}
              onblur={() => saveEdit(p)}
              placeholder="Название"
            />
            <input
              class="text-input"
              bind:value={buffers[p.id].base_url}
              onblur={() => saveEdit(p)}
              placeholder={BASE_URL_HINTS[buffers[p.id].kind]}
            />
            <div class="pwd-row">
              <input
                class="text-input"
                type={showPwdMap[p.id] ? "text" : "password"}
                bind:value={buffers[p.id].key}
                onblur={() => saveEdit(p)}
                placeholder="API key"
              />
              <button
                class="icon-btn"
                onclick={() =>
                  (showPwdMap = { ...showPwdMap, [p.id]: !showPwdMap[p.id] })}
                aria-label="Показать ключ"
              >
                {#if showPwdMap[p.id]}
                  <EyeOff size={15} />
                {:else}
                  <Eye size={15} />
                {/if}
              </button>
            </div>
            <select
              class="text-input"
              bind:value={buffers[p.id].kind}
              onchange={() => saveEdit(p)}
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
    <Plus size={15} /> Добавить прокси
  </button>
</div>

<style>
  .panel {
    position: fixed;
    top: auto;
    left: 8px;
    right: 8px;
    z-index: 200;
    background: var(--bg-2);
    border: 1px solid var(--border);
    border-radius: 14px;
    max-height: 70vh;
    box-shadow: var(--shadow);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  @media (min-width: 600px) {
    .panel {
      position: absolute;
      top: calc(100% + 8px);
      left: auto;
      right: 0;
      min-width: 320px;
      max-width: 380px;
      max-height: none;
    }
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
    max-height: 320px;
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
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 8px 10px;
    background: var(--bg-3);
  }
  .pwd-row {
    display: flex;
    gap: 6px;
  }
  .pwd-row .text-input {
    flex: 1;
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
  .text-input:focus {
    border-color: var(--accent-d);
  }
  .text-input option {
    background: var(--bg-2);
    color: var(--text);
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
