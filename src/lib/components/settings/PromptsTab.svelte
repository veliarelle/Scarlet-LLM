<script lang="ts">
  import { onMount } from "svelte";
  import {
    Plus,
    Trash2,
    ArrowUp,
    ArrowDown,
    ChevronDown,
    Check,
    Save,
    GripVertical,
    Download,
    Upload,
  } from "lucide-svelte";
  import { tr } from "$lib/i18n";
  import { settings } from "$lib/stores/settings";
  import {
    deletePreset,
    exportPreset,
    importPreset,
    loadPresetIntoSettings,
    overwritePreset,
    presetList,
    refreshPresets,
    savePresetFromCurrent,
  } from "$lib/stores/presets";
  import { uid } from "$lib/utils/id";
  import { clickOutside } from "$lib/utils/clickOutside";
  import { DEFAULT_SUMMARIZE_PROMPT, type Prompt, type PromptUtilities } from "$lib/types/settings";
  import type { Role } from "$lib/types/chat";
  import Toggle from "./Toggle.svelte";

  let editingId = $state<string | null>(null);
  let presetOpen = $state(false);
  let showPresetInput = $state(false);
  let newPresetName = $state("");
  let presetSaved = $state(false);
  let presetSavedTimer: ReturnType<typeof setTimeout> | null = null;
  let dragId = $state<string | null>(null);
  let dragOverId = $state<string | null>(null);
  // Флаг: разрешаем DnD только если он начался с drag-handle
  let canDrag = $state(false);

  onMount(() => {
    refreshPresets();
    const reset = () => (canDrag = false);
    document.addEventListener("mouseup", reset);
    document.addEventListener("touchend", reset);
    return () => {
      document.removeEventListener("mouseup", reset);
      document.removeEventListener("touchend", reset);
    };
  });

  async function setPrompts(next: Prompt[]) {
    await settings.patch({ prompts: next });
  }

  async function setUtilities(patch: Partial<PromptUtilities>) {
    await settings.patch({ utilities: { ...$settings.utilities, ...patch } });
  }

  async function addPrompt() {
    const id = uid();
    await setPrompts([
      ...$settings.prompts,
      { id, name: $tr("prompts.newPrompt"), role: "system", content: "", enabled: true },
    ]);
    editingId = id;
  }

  async function update(id: string, patch: Partial<Prompt>) {
    await setPrompts(
      $settings.prompts.map((p) => (p.id === id ? { ...p, ...patch } : p))
    );
  }

  async function remove(id: string) {
    await setPrompts($settings.prompts.filter((p) => p.id !== id));
    if ($settings.utilities.summarize_prompt_id === id) {
      await setUtilities({ summarize_prompt_id: null });
    }
    if (editingId === id) editingId = null;
  }

  async function move(id: string, dir: number) {
    const arr = [...$settings.prompts];
    const i = arr.findIndex((p) => p.id === id);
    const j = i + dir;
    if (i === -1 || j < 0 || j >= arr.length) return;
    [arr[i], arr[j]] = [arr[j], arr[i]];
    await setPrompts(arr);
  }

  function startHandleDrag() {
    canDrag = true;
  }

  function onDragStart(e: DragEvent, id: string) {
    if (!canDrag) {
      e.preventDefault();
      return;
    }
    dragId = id;
    if (e.dataTransfer) {
      e.dataTransfer.effectAllowed = "move";
      // setData нужен для надёжности на разных платформах (особенно Linux/GTK)
      e.dataTransfer.setData("text/plain", id);
    }
  }
  function onDragOver(e: DragEvent, id: string) {
    if (!dragId) return;
    e.preventDefault();
    if (e.dataTransfer) e.dataTransfer.dropEffect = "move";
    if (id !== dragId) dragOverId = id;
  }
  async function onDrop(e: DragEvent, targetId: string) {
    e.preventDefault();
    const movingId = dragId;
    dragId = null;
    dragOverId = null;
    canDrag = false;
    if (!movingId || movingId === targetId) return;
    const arr = [...$settings.prompts];
    const from = arr.findIndex((p) => p.id === movingId);
    const to = arr.findIndex((p) => p.id === targetId);
    if (from === -1 || to === -1) return;
    const [item] = arr.splice(from, 1);
    arr.splice(to, 0, item);
    await setPrompts(arr);
  }
  function onDragEnd() {
    dragId = null;
    dragOverId = null;
    canDrag = false;
  }

  async function loadPreset(id: string) {
    await loadPresetIntoSettings(id);
    presetOpen = false;
  }

  async function savePresetCreate() {
    if (!newPresetName.trim()) return;
    await savePresetFromCurrent(newPresetName.trim(), $settings.prompts, $settings.utilities);
    newPresetName = "";
    showPresetInput = false;
  }

  async function saveToExisting(id: string) {
    if (!confirm($tr("prompts.overwriteConfirm"))) return;
    await overwritePreset(id, $settings.prompts, $settings.utilities);
    presetSaved = true;
    if (presetSavedTimer) clearTimeout(presetSavedTimer);
    presetSavedTimer = setTimeout(() => {
      presetSaved = false;
      presetSavedTimer = null;
    }, 2000);
  }

  async function restoreSummarizePrompt() {
    await setUtilities({ summarize_default_prompt: DEFAULT_SUMMARIZE_PROMPT });
  }

  async function delPreset(id: string) {
    if (!confirm($tr("prompts.deletePresetConfirm"))) return;
    await deletePreset(id);
    if ($settings.active_preset_id === id) {
      await settings.patch({ active_preset_id: null });
    }
  }

  async function onExportPreset() {
    if (!$settings.active_preset_id) return;
    await exportPreset($settings.active_preset_id);
  }

  async function onImportPreset() {
    await importPreset();
  }

  const activePresetName = $derived(
    $presetList.find((p) => p.id === $settings.active_preset_id)?.name ?? $tr("prompts.activePreset")
  );

  const ROLES: Role[] = ["system", "user", "assistant"];
</script>

<div class="prompts-tab">
  <!-- Preset dropdown + save button справа -->
  <div class="preset-row">
    <div class="selector-wrap" use:clickOutside={() => (presetOpen = false)}>
      <button class="selector-btn" onclick={() => (presetOpen = !presetOpen)} type="button">
        <span class="ellipsis">{activePresetName}</span>
        <ChevronDown size={14} color="var(--text-3)" />
      </button>
      {#if presetOpen}
        <div class="dropdown">
          <div class="dropdown-group">{$tr("prompts.savedPresets")}</div>
          {#if $presetList.length === 0}
            <div class="empty">{$tr("prompts.noPresets")}</div>
          {/if}
          {#each $presetList as pr (pr.id)}
            <div class="dropdown-item" class:active={$settings.active_preset_id === pr.id}>
              <button class="preset-pick" onclick={() => loadPreset(pr.id)}>
                <span class="ellipsis">{pr.name}</span>
              </button>
              {#if $settings.active_preset_id === pr.id}
                <Check size={14} color="var(--accent)" />
              {/if}
              <button
                class="preset-icon danger"
                onclick={() => delPreset(pr.id)}
                aria-label={$tr("common.delete")}
              >
                <Trash2 size={13} />
              </button>
            </div>
          {/each}
          <div class="dropdown-divider"></div>
          {#if showPresetInput}
            <div class="save-row">
              <!-- svelte-ignore a11y_autofocus -->
              <input
                class="text-input"
                bind:value={newPresetName}
                onkeydown={(e) => {
                  if (e.key === "Enter") savePresetCreate();
                  if (e.key === "Escape") (showPresetInput = false);
                }}
                placeholder={$tr("prompts.presetNamePlaceholder")}
                autofocus
              />
              <button class="save-btn" onclick={savePresetCreate} aria-label={$tr("common.create")}>
                <Check size={16} />
              </button>
            </div>
          {:else}
            <button class="dropdown-item add" onclick={() => (showPresetInput = true)}>
              <Plus size={14} /> {$tr("prompts.createNew")}
            </button>
          {/if}
        </div>
      {/if}
    </div>

    <button
      class="save-active-btn"
      title={$tr("prompts.exportPresets")}
      onclick={onExportPreset}
      disabled={!$settings.active_preset_id}
      aria-label={$tr("prompts.exportPresets")}
    >
      <Download size={16} />
    </button>

    <button
      class="save-active-btn"
      title={$tr("prompts.importPresets")}
      onclick={onImportPreset}
      aria-label={$tr("prompts.importPresets")}
    >
      <Upload size={16} />
    </button>

    <button
      class="save-active-btn"
      class:saved={presetSaved}
      title={$settings.active_preset_id
        ? $tr("prompts.saveToActiveTitle")
        : $tr("prompts.noActivePreset")}
      onclick={() => $settings.active_preset_id && saveToExisting($settings.active_preset_id)}
      disabled={!$settings.active_preset_id}
      aria-label={$tr("prompts.saveToActive")}
    >
      <Save size={16} />
    </button>
  </div>

  <!-- Prompt list -->
  <div class="prompt-list">
    {#each $settings.prompts as p, i (p.id)}
      <div
        class="prompt-item"
        class:dragging={dragId === p.id}
        class:drag-over={dragOverId === p.id && dragId !== p.id}
        draggable="true"
        ondragstart={(e) => onDragStart(e, p.id)}
        ondragover={(e) => onDragOver(e, p.id)}
        ondrop={(e) => onDrop(e, p.id)}
        ondragend={onDragEnd}
        role="listitem"
      >
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div class="prompt-header" onclick={() => (editingId = editingId === p.id ? null : p.id)}>
          <span
            class="drag-handle"
            title={$tr("prompts.drag")}
            onmousedown={startHandleDrag}
            ontouchstart={startHandleDrag}
            onclick={(e) => e.stopPropagation()}
          >
            <GripVertical size={14} color="var(--text-3)" />
          </span>
          <div class="move" onclick={(e) => e.stopPropagation()}>
            <button onclick={() => move(p.id, -1)} disabled={i === 0} aria-label={$tr("prompts.moveUp")}>
              <ArrowUp size={13} />
            </button>
            <button
              onclick={() => move(p.id, 1)}
              disabled={i === $settings.prompts.length - 1}
              aria-label={$tr("prompts.moveDown")}
            >
              <ArrowDown size={13} />
            </button>
          </div>
          <div onclick={(e) => e.stopPropagation()}>
            <Toggle value={p.enabled} onChange={(v) => update(p.id, { enabled: v })} />
          </div>
          <span class="name">{p.name || $tr("prompts.emptyName")}</span>
          <span class="role-badge role-{p.role}">{p.role}</span>
          <button
            class="del-btn"
            onclick={(e) => {
              e.stopPropagation();
              remove(p.id);
            }}
            aria-label={$tr("common.delete")}
          >
            <Trash2 size={13} color="var(--text-3)" />
          </button>
        </div>

        {#if editingId === p.id}
          <div class="prompt-body">
            <div class="meta">
              <input
                class="text-input"
                value={p.name}
                oninput={(e) => update(p.id, { name: (e.target as HTMLInputElement).value })}
                placeholder={$tr("prompts.namePlaceholder")}
              />
              <select
                class="select"
                value={p.role}
                onchange={(e) => update(p.id, { role: (e.target as HTMLSelectElement).value as Role })}
              >
                {#each ROLES as r (r)}
                  <option value={r}>{r}</option>
                {/each}
              </select>
            </div>
            <textarea
              class="content-area"
              value={p.content}
              oninput={(e) => update(p.id, { content: (e.target as HTMLTextAreaElement).value })}
              placeholder={$tr("prompts.textPlaceholder")}
              rows="5"
            ></textarea>
          </div>
        {/if}
      </div>
    {/each}

    <button class="add-prompt-btn" onclick={addPrompt}>
      <Plus size={16} /> {$tr("prompts.add")}
    </button>
  </div>

  <div class="utilities-section">
    <div class="section-title">{$tr("prompts.utilities")}</div>
    <div class="utility-card">
      <div class="utility-head">
        <div>
          <div class="utility-name">Summarize</div>
          <div class="utility-hint">{$tr("prompts.summarizeHint")}</div>
        </div>
        <select
          class="select utility-select"
          value={$settings.utilities.summarize_prompt_id ?? ""}
          onchange={(e) => {
            const value = (e.target as HTMLSelectElement).value;
            setUtilities({ summarize_prompt_id: value || null });
          }}
        >
          <option value="">{$tr("prompts.defaultUtilityPrompt")}</option>
          {#each $settings.prompts as p (p.id)}
            <option value={p.id}>{p.name || $tr("prompts.emptyName")}</option>
          {/each}
        </select>
      </div>

      {#if !$settings.utilities.summarize_prompt_id}
        <textarea
          class="content-area utility-textarea"
          value={$settings.utilities.summarize_default_prompt}
          oninput={(e) => setUtilities({ summarize_default_prompt: (e.target as HTMLTextAreaElement).value })}
          rows="6"
        ></textarea>
        <div class="utility-actions">
          <button class="restore-btn" onclick={restoreSummarizePrompt}>
            {$tr("prompts.restoreDefault")}
          </button>
        </div>
      {/if}

      <div class="utility-toggle">
        <div>
          <div class="utility-name">{$tr("prompts.autoSummarize")}</div>
          <div class="utility-hint">{$tr("prompts.autoSummarizeHint")}</div>
        </div>
        <Toggle
          value={$settings.utilities.auto_summarize}
          onChange={(v) => setUtilities({ auto_summarize: v })}
        />
      </div>
    </div>
  </div>
</div>

<style>
  .prompts-tab {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .preset-row {
    display: flex;
    gap: 8px;
    align-items: stretch;
  }
  .selector-wrap {
    position: relative;
    flex: 1;
  }
  .save-active-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 36px;
    border-radius: 8px;
    background: var(--bg-3);
    border: 1px solid var(--border);
    color: var(--text-2);
    transition: background 0.12s, color 0.12s, opacity 0.12s;
  }
  .save-active-btn:hover:not(:disabled) {
    background: var(--bg-4);
    color: var(--text);
  }
  .save-active-btn.saved:not(:disabled) {
    background: color-mix(in srgb, #22c55e 78%, var(--bg-3));
    border-color: color-mix(in srgb, #22c55e 72%, var(--border));
    color: white;
  }
  .save-active-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
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
    width: 100%;
  }
  .selector-btn:hover {
    background: var(--bg-4);
    color: var(--text);
  }
  .ellipsis {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    text-align: left;
  }

  .dropdown {
    position: absolute;
    top: calc(100% + 6px);
    left: 0;
    right: 0;
    z-index: 110;
    background: var(--bg-3);
    border: 1px solid var(--border);
    border-radius: 12px;
    padding: 6px;
    max-height: 360px;
    overflow-y: auto;
    box-shadow: var(--shadow);
  }
  .dropdown-group {
    padding: 6px 10px 4px;
    font-size: 11px;
    color: var(--text-3);
    text-transform: uppercase;
    letter-spacing: 0.08em;
  }
  .dropdown-divider {
    height: 1px;
    background: var(--border);
    margin: 4px 0;
  }
  .dropdown-item {
    display: flex;
    align-items: center;
    gap: 4px;
    width: 100%;
    padding: 4px 6px 4px 10px;
    border-radius: 7px;
    color: var(--text-2);
    font-size: 13px;
    transition: background 0.1s;
  }
  .dropdown-item:hover {
    background: var(--bg-4);
  }
  .dropdown-item.active {
    color: var(--text);
  }
  .dropdown-item.add {
    cursor: pointer;
  }
  .preset-pick {
    flex: 1;
    text-align: left;
    color: inherit;
    padding: 3px 0;
  }
  .preset-icon {
    color: var(--text-3);
    opacity: 0.7;
    padding: 4px;
    border-radius: 5px;
    display: flex;
  }
  .preset-icon:hover {
    opacity: 1;
    background: var(--bg-4);
    color: var(--text);
  }
  .preset-icon.danger:hover {
    color: var(--danger);
  }

  .empty {
    padding: 8px 10px;
    color: var(--text-3);
    font-size: 12px;
  }
  .save-row {
    padding: 6px;
    display: flex;
    gap: 6px;
  }
  .save-row .text-input {
    flex: 1;
  }
  .save-btn {
    padding: 4px 8px;
    color: var(--accent);
    display: flex;
    align-items: center;
  }

  .prompt-list {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .prompt-item {
    background: var(--bg-3);
    border: 1px solid var(--border);
    border-radius: 10px;
    overflow: hidden;
  }
  .prompt-item.dragging {
    opacity: 0.4;
  }
  .prompt-item.drag-over {
    outline: 2px solid var(--accent-d);
    outline-offset: 1px;
  }
  .prompt-header {
    display: flex;
    align-items: center;
    gap: 7px;
    padding: 8px 10px;
    cursor: pointer;
    user-select: none;
  }
  .prompt-header:hover {
    background: var(--bg-4);
  }
  .drag-handle {
    display: flex;
    cursor: grab;
    padding: 2px 4px;
    border-radius: 4px;
    user-select: none;
    -webkit-user-select: none;
  }
  .drag-handle:hover {
    background: var(--border);
  }
  .drag-handle:active {
    cursor: grabbing;
  }
  .move {
    display: flex;
    gap: 1px;
  }
  .move button {
    color: var(--text-3);
    padding: 3px;
    border-radius: 4px;
    display: flex;
  }
  .move button:hover:not(:disabled) {
    background: var(--bg-4);
    color: var(--text);
  }
  .move button:disabled {
    opacity: 0.25;
  }
  .name {
    flex: 1;
    font-size: 13px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .role-badge {
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.07em;
    padding: 2px 7px;
    border-radius: 10px;
    font-weight: 600;
  }
  .role-system {
    background: oklch(22% 0.05 240);
    color: oklch(65% 0.12 240);
  }
  .role-user {
    background: oklch(22% 0.05 145);
    color: oklch(65% 0.12 145);
  }
  .role-assistant {
    background: oklch(22% 0.06 18);
    color: var(--accent);
  }
  .del-btn {
    opacity: 0.5;
    display: flex;
  }
  .del-btn:hover {
    opacity: 1;
  }

  .prompt-body {
    padding: 0 10px 10px;
    display: flex;
    flex-direction: column;
    gap: 7px;
  }
  .meta {
    display: flex;
    gap: 7px;
  }
  .meta .text-input {
    flex: 1;
  }
  .text-input {
    background: var(--bg-3);
    border: 1px solid var(--border);
    border-radius: 7px;
    padding: 6px 10px;
    color: var(--text);
    font-size: 13px;
    width: 100%;
  }
  .text-input:focus {
    border-color: var(--accent-d);
  }
  .select {
    background: var(--bg-3);
    border: 1px solid var(--border);
    border-radius: 7px;
    padding: 5px 8px;
    color: var(--text);
  }
  .select option {
    background: var(--bg-2);
    color: var(--text);
  }
  .content-area {
    width: 100%;
    background: var(--bg-4);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 8px 10px;
    font-size: 13px;
    line-height: 1.5;
    color: var(--text);
    min-height: 110px;
    resize: vertical;
  }
  .content-area:focus {
    border-color: var(--accent-d);
  }

  .add-prompt-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    padding: 10px;
    border-radius: 10px;
    font-size: 13px;
    background: none;
    border: 1.5px dashed var(--border);
    color: var(--text-3);
    transition: all 0.12s;
  }
  .add-prompt-btn:hover {
    border-color: var(--accent-d);
    color: var(--text-2);
    background: var(--bg-3);
  }

  .utilities-section {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding-top: 6px;
  }
  .section-title {
    font-size: 11px;
    color: var(--text-3);
    text-transform: uppercase;
    letter-spacing: 0.08em;
    padding: 0 2px;
  }
  .utility-card {
    background: var(--bg-3);
    border: 1px solid var(--border);
    border-radius: 10px;
    padding: 10px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .utility-head {
    display: flex;
    align-items: flex-start;
    gap: 10px;
  }
  .utility-name {
    font-size: 13px;
    color: var(--text);
    font-weight: 600;
  }
  .utility-hint {
    margin-top: 3px;
    font-size: 12px;
    color: var(--text-3);
    line-height: 1.4;
  }
  .utility-select {
    width: min(210px, 45%);
    flex-shrink: 0;
  }
  .utility-textarea {
    min-height: 140px;
  }
  .utility-actions {
    display: flex;
    justify-content: flex-end;
  }
  .utility-toggle {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding-top: 2px;
  }
  .restore-btn {
    padding: 6px 10px;
    border-radius: 7px;
    background: var(--bg-4);
    color: var(--text-2);
    font-size: 12px;
  }
  .restore-btn:hover {
    color: var(--text);
  }

  @media (max-width: 520px) {
    .utility-head {
      flex-direction: column;
    }
    .utility-select {
      width: 100%;
    }
  }
</style>
