<script lang="ts">
  import { Paperclip, Send, Settings as SettingsIcon, Square, X, FileText, Image as ImageIcon } from "lucide-svelte";
  import { settingsOpen } from "$lib/stores/ui";
  import type { Attachment } from "$lib/types/chat";
  import { uid } from "$lib/utils/id";

  let {
    onSend,
    onStop,
    disabled = false,
    busy = false,
    canStop = false,
    canResend = false,
  }: {
    onSend: (text: string, attachments: Attachment[]) => void | Promise<void>;
    onStop?: () => void;
    disabled?: boolean;
    busy?: boolean;
    canStop?: boolean;
    canResend?: boolean;
  } = $props();

  let text = $state("");
  let attachments = $state<Attachment[]>([]);
  let submitting = $state(false);
  let textareaEl: HTMLTextAreaElement | undefined = $state();
  let fileInputEl: HTMLInputElement | undefined = $state();

  $effect(() => {
    void text;
    if (!textareaEl) return;
    textareaEl.style.height = "auto";
    textareaEl.style.height = Math.min(textareaEl.scrollHeight, 240) + "px";
  });

  const canSend = $derived(
    !disabled && !busy && !submitting && (text.trim().length > 0 || canResend || attachments.length > 0)
  );

  async function submit() {
    if (!canSend) return;
    submitting = true;
    const nextText = text.trim();
    const nextAttachments = attachments;
    text = "";
    attachments = [];
    try {
      await onSend(nextText, nextAttachments);
    } finally {
      submitting = false;
    }
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      submit();
    }
  }

  function openFilePicker() {
    fileInputEl?.click();
  }

  async function onFileChange(e: Event) {
    const files = (e.target as HTMLInputElement).files;
    if (!files) return;
    for (const file of files) {
      const data = await readAsBase64(file);
      attachments = [
        ...attachments,
        { id: uid(), name: file.name, mimeType: file.type || "application/octet-stream", data },
      ];
    }
    if (fileInputEl) fileInputEl.value = "";
  }

  function readAsBase64(file: File): Promise<string> {
    return new Promise((resolve, reject) => {
      const reader = new FileReader();
      reader.onload = () => {
        const result = reader.result as string;
        resolve(result.split(",")[1] ?? "");
      };
      reader.onerror = reject;
      reader.readAsDataURL(file);
    });
  }

  function removeAttachment(id: string) {
    attachments = attachments.filter((a) => a.id !== id);
  }
</script>

<div class="input-outer">
  <div class="input-wrap">
    {#if attachments.length > 0}
      <div class="att-row">
        {#each attachments as att (att.id)}
          <div class="att-chip">
            <span class="att-icon">
              {#if att.mimeType.startsWith("image/")}
                <ImageIcon size={11} />
              {:else}
                <FileText size={11} />
              {/if}
            </span>
            <span class="att-name" title={att.name}>{att.name}</span>
            <button class="att-rm" onclick={() => removeAttachment(att.id)} title="Убрать">
              <X size={10} />
            </button>
          </div>
        {/each}
      </div>
    {/if}
    <div class="input-box">
      <button
        class="icon-btn"
        title="Прикрепить файл"
        onclick={openFilePicker}
      >
        <Paperclip size={18} />
      </button>
      <textarea
        bind:this={textareaEl}
        bind:value={text}
        rows="1"
        placeholder="Сообщение Scarlet…"
        onkeydown={onKey}
        disabled={disabled && !busy}
        class="textarea"
      ></textarea>
      <div class="input-right">
        <button
          class="icon-btn"
          title="Настройки"
          onclick={() => settingsOpen.set(true)}
        >
          <SettingsIcon size={18} />
        </button>

        {#if busy}
          <button
            class="send-btn stop"
            class:active={canStop}
            title={canStop ? "Остановить" : "Ожидание ответа"}
            onclick={() => onStop?.()}
            disabled={!canStop}
            aria-label="Остановить"
          >
            <Square
              size={14}
              fill={canStop ? "white" : "var(--text-3)"}
              color={canStop ? "white" : "var(--text-3)"}
            />
          </button>
        {:else}
          <button
            class="send-btn"
            class:active={canSend}
            onclick={submit}
            disabled={!canSend}
            aria-label="Отправить"
          >
            <Send size={16} color={canSend ? "white" : "var(--text-3)"} />
          </button>
        {/if}
      </div>
    </div>
    <!-- скрытый file picker -->
    <input
      bind:this={fileInputEl}
      type="file"
      multiple
      style="display:none"
      onchange={onFileChange}
    />
  </div>
</div>

<style>
  .input-outer {
    padding: 0 24px 18px;
    width: 100%;
  }
  @media (min-width: 768px) {
    .input-outer {
      padding-bottom: 20px;
    }
  }
  .input-wrap {
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  /* Attachment chips */
  .att-row {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    padding: 0 4px 2px;
  }
  .att-chip {
    display: flex;
    align-items: center;
    gap: 5px;
    background: var(--bg-4);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 3px 6px 3px 8px;
    max-width: 180px;
    font-size: 12px;
    color: var(--text-2);
  }
  .att-icon {
    display: flex;
    align-items: center;
    color: var(--accent);
    flex-shrink: 0;
  }
  .att-name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    flex: 1;
    min-width: 0;
  }
  .att-rm {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 16px;
    height: 16px;
    border-radius: 4px;
    color: var(--text-3);
    flex-shrink: 0;
    transition: background 0.1s, color 0.1s;
  }
  .att-rm:hover {
    background: var(--border);
    color: var(--text);
  }

  .input-box {
    display: flex;
    align-items: flex-end;
    gap: 8px;
    background: var(--bg-3);
    border: 1px solid var(--border);
    border-radius: 14px;
    padding: 8px 8px 8px 12px;
    transition: border-color 0.2s;
    width: 100%;
  }
  .input-box:focus-within {
    border-color: var(--accent-d);
  }
  .textarea {
    flex: 1;
    font-size: 14px;
    line-height: 1.6;
    max-height: 240px;
    overflow-y: auto;
    padding-top: 2px;
    color: var(--text);
  }
  .textarea::placeholder {
    color: var(--text-3);
  }
  .icon-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    border-radius: 8px;
    color: var(--text-3);
    transition: background 0.12s, color 0.12s;
  }
  .icon-btn:hover {
    background: var(--bg-4);
    color: var(--text-2);
  }
  .input-right {
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .send-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    border-radius: 8px;
    background: var(--bg-4);
    transition: background 0.15s;
  }
  .send-btn.active {
    background: var(--accent);
  }
  .send-btn.active:hover {
    background: var(--accent-h);
  }
  .send-btn:disabled {
    cursor: default;
  }
  .send-btn.stop {
    background: var(--bg-4);
    cursor: default;
  }
  .send-btn.stop.active {
    background: var(--accent);
    cursor: pointer;
  }
  .send-btn.stop.active:hover {
    background: var(--accent-h);
  }
</style>
