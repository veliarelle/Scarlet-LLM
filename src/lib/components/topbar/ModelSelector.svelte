<script lang="ts">
  import { Sparkles, ChevronDown, Check, Pencil, RotateCw } from "lucide-svelte";
  import { tr } from "$lib/i18n";
  import { settings } from "$lib/stores/settings";
  import { models } from "$lib/stores/models";
  import { clickOutside } from "$lib/utils/clickOutside";

  let open = $state(false);
  let customMode = $state(false);
  let customVal = $state("");
  let search = $state("");

  function close() {
    open = false;
    customMode = false;
    search = "";
  }

  async function pick(m: string) {
    await settings.patch({ active_model: m });
    close();
  }

  async function commitCustom() {
    const v = customVal.trim();
    if (!v) return;
    await pick(v);
    customVal = "";
  }

  async function refresh() {
    if ($settings.active_proxy_id) await models.load($settings.active_proxy_id);
  }

  function modelMatches(m: { id: string; name?: string | null }, query: string): boolean {
    const q = query.trim().toLowerCase();
    if (!q) return true;
    return m.id.toLowerCase().includes(q) || (m.name ?? "").toLowerCase().includes(q);
  }

  // Группируем модели по префиксу провайдера, если возможно
  function groupModels(list: { id: string; name?: string | null }[]) {
    const groups: Record<string, { id: string; name?: string | null }[]> = {};
    const order: string[] = [];
    for (const m of list) {
      const id = m.id.toLowerCase();
      let g = "Other";
      if (id.includes("gpt") || id.startsWith("o1") || id.startsWith("o3")) g = "OpenAI";
      else if (id.includes("claude")) g = "Anthropic";
      else if (id.includes("gemini") || id.includes("google")) g = "Google";
      else if (id.includes("llama") || id.includes("meta")) g = "Meta";
      else if (id.includes("mistral") || id.includes("codestral") || id.includes("ministral")) g = "Mistral";
      else if (id.includes("deepseek")) g = "DeepSeek";
      else if (id.includes("grok") || id.includes("xai")) g = "xAI";
      else if (id.includes("glm") || id.includes("zhipu")) g = "GLM";
      if (!groups[g]) {
        groups[g] = [];
        order.push(g);
      }
      groups[g].push(m);
    }
    return order.map((g) => ({ group: g, models: groups[g] }));
  }

  const filteredModels = $derived($models.list.filter((m) => modelMatches(m, search)));
  const grouped = $derived(groupModels(filteredModels));

  function shorten(m: string): string {
    return m.length > 22 ? m.slice(0, 22) + "…" : m;
  }

  const display = $derived($settings.active_model ?? $tr("model.fallback"));
</script>

<div class="selector-wrap" use:clickOutside={close}>
  <button class="selector-btn" onclick={() => (open = !open)} title={display}>
    <Sparkles size={13} color="var(--accent)" />
    <span>{shorten(display)}</span>
    <ChevronDown size={13} color="var(--text-3)" />
  </button>

  {#if open}
    <div class="dropdown">
      <div class="dropdown-top">
        <button
          class="dropdown-item refresh-item"
          onclick={refresh}
          disabled={$models.loading || !$settings.active_proxy_id}
          title={$settings.active_proxy_id ? $tr("model.refresh") : $tr("model.noProxy")}
        >
          <span class="refresh-icon" class:spin={$models.loading}>
            <RotateCw size={13} />
          </span>
          <span>{$tr("model.refresh")}</span>
        </button>
      </div>

      <div class="search-wrap">
        <input
          class="search-input"
          bind:value={search}
          placeholder={$tr("model.search")}
          autocomplete="off"
          spellcheck="false"
        />
      </div>

      {#if $models.loading}
        <div class="dropdown-status">{$tr("model.loading")}</div>
      {:else if $models.error}
        <div class="dropdown-status err" title={$models.error}>{$tr("model.error")}</div>
      {:else if $models.list.length === 0}
        <div class="dropdown-status">{$tr("model.empty")}</div>
      {:else if grouped.length === 0}
        <div class="dropdown-status">{$tr("model.noResults")}</div>
      {:else}
        {#each grouped as g (g.group)}
          <div class="dropdown-group">{g.group}</div>
          {#each g.models as m (m.id)}
            <button
              class="dropdown-item"
              class:active={$settings.active_model === m.id}
              onclick={() => pick(m.id)}
            >
              <span class="m-name">{m.name ?? m.id}</span>
              {#if m.name && m.name !== m.id}
                <span class="m-id">{m.id}</span>
              {/if}
              {#if $settings.active_model === m.id}
                <Check size={13} color="var(--accent)" />
              {/if}
            </button>
          {/each}
        {/each}
      {/if}

      <div class="dropdown-divider"></div>

      {#if customMode}
        <div class="dropdown-custom">
          <input
            class="text-input"
            bind:value={customVal}
            onkeydown={(e) => {
              if (e.key === "Enter") commitCustom();
              if (e.key === "Escape") (customMode = false);
            }}
            placeholder="model-id…"
          />
          <button class="dropdown-item" onclick={commitCustom}>
            <Check size={14} /> {$tr("model.use")}
          </button>
        </div>
      {:else}
        <button class="dropdown-item" onclick={() => (customMode = true)}>
          <Pencil size={14} /> {$tr("model.custom")}
        </button>
      {/if}
    </div>
  {/if}
</div>

<style>
  .selector-wrap {
    position: relative;
    flex: 0 1 auto;
    min-width: 0;
  }
  .selector-btn {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 10px;
    border-radius: 8px;
    background: var(--bg-3);
    border: 1px solid var(--border);
    color: var(--text-2);
    font-size: 13px;
    white-space: nowrap;
    transition: background 0.12s, color 0.12s;
    max-width: min(200px, 34vw);
    min-width: 0;
  }
  .selector-btn span {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .selector-btn:hover {
    background: var(--bg-4);
    color: var(--text);
  }

  .dropdown {
    position: fixed;
    top: auto;
    left: 8px;
    right: 8px;
    z-index: 200;
    background: var(--bg-3);
    border: 1px solid var(--border);
    border-radius: 12px;
    padding: 6px;
    min-width: 230px;
    max-height: min(360px, 55vh);
    overflow-y: auto;
    box-shadow: var(--shadow);
  }
  @media (min-width: 600px) {
    .dropdown {
      position: absolute;
      top: calc(100% + 6px);
      left: 0;
      right: auto;
      max-height: 360px;
    }
  }
  .dropdown-group {
    padding: 6px 10px 4px;
    font-size: 11px;
    color: var(--text-3);
    text-transform: uppercase;
    letter-spacing: 0.08em;
  }
  .dropdown-top {
    padding-bottom: 4px;
  }
  .dropdown-status {
    padding: 8px 10px;
    color: var(--text-3);
    font-size: 12px;
  }
  .dropdown-status.err {
    color: var(--danger);
  }
  .search-wrap {
    padding: 4px 0 6px;
  }
  .search-input {
    width: 100%;
    height: 32px;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 7px;
    padding: 0 10px;
    color: var(--text);
    font-size: 12px;
  }
  .search-input:focus {
    border-color: var(--accent-d);
  }
  .dropdown-item {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 7px 10px;
    border-radius: 7px;
    color: var(--text-2);
    font-size: 13px;
    transition: background 0.1s;
    text-align: left;
  }
  .dropdown-item:hover {
    background: var(--bg-4);
    color: var(--text);
  }
  .dropdown-item.active {
    color: var(--text);
  }
  .dropdown-item:disabled {
    cursor: default;
    opacity: 0.55;
  }
  .dropdown-item:disabled:hover {
    background: transparent;
    color: var(--text-2);
  }
  .refresh-item {
    color: var(--text);
    background: var(--bg);
    border: 1px solid var(--border);
  }
  .refresh-item:hover {
    background: var(--bg-4);
  }
  .refresh-icon {
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .spin {
    animation: spin 0.8s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  .dropdown-item .m-name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .m-id {
    max-width: 120px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--text-3);
    font-size: 11px;
  }
  .dropdown-divider {
    height: 1px;
    background: var(--border);
    margin: 4px 0;
  }
  .dropdown-custom {
    padding: 6px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .text-input {
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 7px;
    padding: 6px 10px;
    width: 100%;
    color: var(--text);
  }
  .text-input:focus {
    border-color: var(--accent-d);
  }
</style>
