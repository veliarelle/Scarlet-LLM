<script lang="ts">
  import { Check, Edit3, Trash2, X } from "lucide-svelte";
  import { tr } from "$lib/i18n";
  import type { ChatSummary } from "$lib/types/chat";

  let {
    summary,
    onSave,
    onDelete,
  }: {
    summary: ChatSummary;
    onSave: (content: string) => void | Promise<void>;
    onDelete: () => void | Promise<void>;
  } = $props();

  let open = $state(false);
  let editing = $state(false);
  let draft = $state("");

  $effect(() => {
    if (!editing) draft = summary.content;
  });

  async function save() {
    const next = draft.trim();
    if (!next) return;
    await onSave(next);
    editing = false;
  }

  function cancel() {
    draft = summary.content;
    editing = false;
  }
</script>

<div class="summary-wrap">
  <button class="summary-line" onclick={() => (open = !open)}>
    <span class="line"></span>
    <span class="label">{$tr("chat.summaryBoundary")}</span>
    <span class="line"></span>
  </button>

  {#if open}
    <div class="summary-panel">
      <div class="summary-note">{$tr("chat.summaryNotIncluded")}</div>
      <div class="prompt-label">{$tr("chat.summaryPrompt")}</div>
      <div class="prompt-box">{summary.prompt}</div>

      {#if editing}
        <textarea class="summary-edit" bind:value={draft} rows="8"></textarea>
        <div class="actions">
          <button class="action-btn" onclick={cancel}>
            <X size={14} /> {$tr("common.cancel")}
          </button>
          <button class="action-btn primary" onclick={save} disabled={!draft.trim()}>
            <Check size={14} /> {$tr("chat.summarySave")}
          </button>
        </div>
      {:else}
        <div class="summary-text">{summary.content}</div>
        <div class="actions">
          <button class="action-btn danger" onclick={onDelete}>
            <Trash2 size={14} /> {$tr("chat.summaryDelete")}
          </button>
          <button class="action-btn" onclick={() => (editing = true)}>
            <Edit3 size={14} /> {$tr("chat.summaryEdit")}
          </button>
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .summary-wrap {
    width: min(900px, 100%);
    align-self: center;
    padding: 10px 0 18px;
  }
  .summary-line {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 10px;
    color: var(--text-3);
    font-size: 12px;
  }
  .summary-line:hover {
    color: var(--text-2);
  }
  .line {
    height: 1px;
    flex: 1;
    background: var(--border);
  }
  .label {
    padding: 4px 9px;
    border-radius: 999px;
    background: var(--bg-3);
    border: 1px solid var(--border);
    white-space: nowrap;
  }
  .summary-panel {
    margin-top: 10px;
    background: var(--bg-3);
    border: 1px solid var(--border);
    border-radius: 10px;
    padding: 12px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .summary-note {
    color: var(--text-3);
    font-size: 12px;
    line-height: 1.45;
  }
  .prompt-label {
    color: var(--text-3);
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.08em;
  }
  .prompt-box,
  .summary-text,
  .summary-edit {
    background: var(--bg-4);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 9px 10px;
    color: var(--text-2);
    font-size: 13px;
    line-height: 1.55;
    white-space: pre-wrap;
    word-break: break-word;
  }
  .prompt-box {
    max-height: 140px;
    overflow-y: auto;
    color: var(--text-3);
  }
  .summary-edit {
    width: 100%;
    min-height: 170px;
    resize: vertical;
    color: var(--text);
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
  .action-btn {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 7px 10px;
    border-radius: 8px;
    background: var(--bg-4);
    color: var(--text-2);
    font-size: 12px;
  }
  .action-btn:hover:not(:disabled) {
    color: var(--text);
  }
  .action-btn.primary {
    background: var(--accent);
    color: white;
  }
  .action-btn.danger:hover {
    color: var(--danger);
  }
  .action-btn:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }
</style>
