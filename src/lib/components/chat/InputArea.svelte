<script lang="ts">
  import { Paperclip, Send, Settings as SettingsIcon, Square, X, FileText, Image as ImageIcon } from "lucide-svelte";
  import { onMount } from "svelte";
  import { getCurrentWebview } from "@tauri-apps/api/webview";
  import { api } from "$lib/api/invoke";
  import { tr } from "$lib/i18n";
  import { settingsOpen } from "$lib/stores/ui";
  import type { Attachment } from "$lib/types/chat";
  import { clickOutside } from "$lib/utils/clickOutside";
  import { uid } from "$lib/utils/id";

  let {
    onSend,
    onStop,
    disabled = false,
    busy = false,
    canStop = false,
    canResend = false,
    contextUsed = 0,
    contextWindow = 0,
    onSummarize,
    canSummarize = false,
  }: {
    onSend: (text: string, attachments: Attachment[]) => void | Promise<void>;
    onStop?: () => void;
    onSummarize?: () => void | Promise<void>;
    disabled?: boolean;
    busy?: boolean;
    canStop?: boolean;
    canResend?: boolean;
    contextUsed?: number;
    contextWindow?: number;
    canSummarize?: boolean;
  } = $props();

  let text = $state("");
  let attachments = $state<Attachment[]>([]);
  let submitting = $state(false);
  let textareaEl: HTMLTextAreaElement | undefined = $state();
  let fileInputEl: HTMLInputElement | undefined = $state();
  let showContextNumber = $state(false);
  let dragDepth = $state(0);
  let dropActive = $state(false);

  $effect(() => {
    void text;
    if (!textareaEl) return;
    textareaEl.style.height = "auto";
    textareaEl.style.height = Math.min(textareaEl.scrollHeight, 240) + "px";
  });

  onMount(() => {
    let nativeUnlisten: (() => void) | null = null;
    void getCurrentWebview().onDragDropEvent((event) => {
      const payload = event.payload;
      if (payload.type === "over" || payload.type === "enter") {
        if (!$settingsOpen) dropActive = true;
        return;
      }
      if (payload.type === "drop") {
        dragDepth = 0;
        dropActive = false;
        if ($settingsOpen) return;
        void addDroppedPaths(payload.paths);
        return;
      }
      dragDepth = 0;
      dropActive = false;
    }).then((unlisten) => {
      nativeUnlisten = unlisten;
    }).catch(() => {
      nativeUnlisten = null;
    });

    window.addEventListener("dragenter", onWindowDragEnter);
    window.addEventListener("dragover", onWindowDragOver);
    window.addEventListener("dragleave", onWindowDragLeave);
    window.addEventListener("drop", onWindowDrop);
    return () => {
      window.removeEventListener("dragenter", onWindowDragEnter);
      window.removeEventListener("dragover", onWindowDragOver);
      window.removeEventListener("dragleave", onWindowDragLeave);
      window.removeEventListener("drop", onWindowDrop);
      nativeUnlisten?.();
    };
  });

  const canSend = $derived(
    !disabled && !busy && !submitting && (text.trim().length > 0 || canResend || attachments.length > 0)
  );
  const contextLimit = $derived(Math.max(0, Math.floor(Number(contextWindow) || 0)));
  const contextValue = $derived(Math.max(0, Math.floor(Number(contextUsed) || 0)));
  const contextRatio = $derived(contextLimit > 0 ? Math.min(1, contextValue / contextLimit) : 0);
  const contextPercent = $derived(Math.round(contextRatio * 100));
  const contextTitle = $derived(
    $tr("input.contextUsage", {
      used: contextValue.toLocaleString(),
      limit: contextLimit > 0 ? contextLimit.toLocaleString() : "∞",
    })
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
    await addFiles(files);
    if (fileInputEl) fileInputEl.value = "";
  }

  async function addFiles(files: Iterable<File>) {
    const raw: { name: string; mimeType: string; data: string }[] = [];
    for (const file of files) {
      raw.push({
        name: file.name,
        mimeType: file.type || "",
        data: await readAsBase64(file),
      });
    }
    if (raw.length === 0) return;

    try {
      const prepared = await api.prepareAttachments(raw);
      if (prepared.length > 0) attachments = [...attachments, ...prepared];
    } catch (e) {
      console.error("Failed to prepare attachments", e);
      attachments = [
        ...attachments,
        ...raw.map((file) => ({
          id: uid(),
          name: file.name,
          mimeType: file.mimeType || "application/octet-stream",
          data: file.data,
          text: null,
        })),
      ];
    }
  }

  async function addDroppedPaths(paths: string[]) {
    if (paths.length === 0) return;
    try {
      const dropped = await api.readDroppedFiles(paths);
      if (dropped.length > 0) attachments = [...attachments, ...dropped];
    } catch (e) {
      console.error("Failed to read dropped files", e);
    }
  }

  function hasDraggedFiles(e: DragEvent): boolean {
    return Array.from(e.dataTransfer?.types ?? []).includes("Files");
  }

  function onDragEnter(e: DragEvent) {
    if (!hasDraggedFiles(e)) return;
    e.preventDefault();
    e.stopPropagation();
    dragDepth += 1;
    dropActive = true;
  }

  function onDragOver(e: DragEvent) {
    if (!hasDraggedFiles(e)) return;
    e.preventDefault();
    e.stopPropagation();
    if (e.dataTransfer) e.dataTransfer.dropEffect = "copy";
    dropActive = true;
  }

  function onDragLeave(e: DragEvent) {
    if (!hasDraggedFiles(e)) return;
    e.preventDefault();
    e.stopPropagation();
    dragDepth = Math.max(0, dragDepth - 1);
    if (dragDepth === 0) dropActive = false;
  }

  async function onDrop(e: DragEvent) {
    if (!hasDraggedFiles(e)) return;
    e.preventDefault();
    e.stopPropagation();
    dragDepth = 0;
    dropActive = false;
    await addFiles(Array.from(e.dataTransfer?.files ?? []));
  }

  function onWindowDragEnter(e: DragEvent) {
    if (!hasDraggedFiles(e)) return;
    e.preventDefault();
    if ($settingsOpen) return;
    dragDepth += 1;
    dropActive = true;
  }

  function onWindowDragOver(e: DragEvent) {
    if (!hasDraggedFiles(e)) return;
    e.preventDefault();
    if (e.dataTransfer) e.dataTransfer.dropEffect = $settingsOpen ? "none" : "copy";
    if ($settingsOpen) return;
    dropActive = true;
  }

  function onWindowDragLeave(e: DragEvent) {
    if (!hasDraggedFiles(e)) return;
    if ($settingsOpen) return;
    dragDepth = Math.max(0, dragDepth - 1);
    if (
      dragDepth === 0 ||
      e.clientX <= 0 ||
      e.clientY <= 0 ||
      e.clientX >= window.innerWidth ||
      e.clientY >= window.innerHeight
    ) {
      dropActive = false;
    }
  }

  async function onWindowDrop(e: DragEvent) {
    if (!hasDraggedFiles(e)) return;
    e.preventDefault();
    dragDepth = 0;
    dropActive = false;
    if ($settingsOpen) return;
    await addFiles(Array.from(e.dataTransfer?.files ?? []));
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

<div
  class="input-outer"
  ondragenter={onDragEnter}
  ondragover={onDragOver}
  ondragleave={onDragLeave}
  ondrop={onDrop}
  role="group"
  aria-label={$tr("input.attachFile")}
>
  <div class="input-wrap" class:drop-active={dropActive}>
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
            <button class="att-rm" onclick={() => removeAttachment(att.id)} title={$tr("input.removeAttachment")}>
              <X size={10} />
            </button>
          </div>
        {/each}
      </div>
    {/if}
    <div class="input-box">
      <button
        class="icon-btn"
        title={$tr("input.attachFile")}
        onclick={openFilePicker}
      >
        <Paperclip size={18} />
      </button>
      <textarea
        bind:this={textareaEl}
        bind:value={text}
        rows="1"
        placeholder={$tr("input.placeholder")}
        onkeydown={onKey}
        disabled={disabled && !busy}
        class="textarea"
      ></textarea>
      <div class="input-right">
        <div class="context-wrap" use:clickOutside={() => (showContextNumber = false)}>
          <button
            class="context-meter"
            class:warn={contextRatio >= 0.8}
            class:full={contextRatio >= 0.98}
            style={`--context-fill: ${contextPercent}%;`}
            title={contextTitle}
            onclick={() => (showContextNumber = !showContextNumber)}
            aria-label={contextTitle}
          >
            {#if showContextNumber}
              <span class="context-number">{contextLimit > 0 ? `${contextPercent}%` : "∞"}</span>
            {:else}
              <span class="context-dot"></span>
            {/if}
          </button>
          {#if showContextNumber}
            <div class="context-popover">
              <div class="context-exact">
                <span>{contextValue.toLocaleString()} / {contextLimit > 0 ? contextLimit.toLocaleString() : "∞"}</span>
                <span class="context-percent">{contextLimit > 0 ? `${contextPercent}%` : "∞"}</span>
              </div>
              <button
                class="context-action"
                onclick={() => {
                  showContextNumber = false;
                  onSummarize?.();
                }}
                disabled={!canSummarize}
              >
                {$tr("input.summarize")}
              </button>
            </div>
          {/if}
        </div>
        <button
          class="icon-btn"
          title={$tr("settings.title")}
          onclick={() => settingsOpen.set(true)}
        >
          <SettingsIcon size={18} />
        </button>

        {#if busy}
          <button
            class="send-btn stop"
            class:active={canStop}
            title={canStop ? $tr("input.stop") : $tr("input.waiting")}
            onclick={() => onStop?.()}
            disabled={!canStop}
            aria-label={$tr("input.stop")}
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
            aria-label={$tr("input.send")}
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
    flex-shrink: 0;
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
    position: relative;
  }
  .input-wrap.drop-active .input-box {
    border-color: var(--accent);
    background: color-mix(in srgb, var(--bg-3) 82%, var(--accent));
    box-shadow: 0 0 0 2px color-mix(in srgb, var(--accent) 28%, transparent);
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
  .context-wrap {
    position: relative;
    flex-shrink: 0;
  }
  .context-meter {
    --meter-color: var(--accent);
    --context-fill: 0%;
    width: 32px;
    height: 32px;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    background:
      radial-gradient(circle at center, var(--bg-3) 56%, transparent 57%),
      conic-gradient(var(--meter-color) var(--context-fill), var(--bg-4) 0);
    color: var(--text-2);
    border: 1px solid var(--border);
    transition: border-color 0.12s, color 0.12s, background 0.12s;
  }
  .context-meter:hover {
    border-color: var(--accent-d);
    color: var(--text);
  }
  .context-meter.warn {
    --meter-color: oklch(72% 0.16 70);
  }
  .context-meter.full {
    --meter-color: var(--danger);
  }
  .context-dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--meter-color);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--meter-color) 16%, transparent);
  }
  .context-number {
    font-size: 9px;
    font-family: "JetBrains Mono", monospace;
    font-weight: 600;
    line-height: 1;
    font-variant-numeric: tabular-nums;
  }
  .context-popover {
    position: absolute;
    right: 0;
    bottom: calc(100% + 8px);
    min-width: 150px;
    padding: 8px;
    border-radius: 9px;
    background: var(--bg-2);
    border: 1px solid var(--border);
    box-shadow: var(--shadow);
    color: var(--text-2);
    display: flex;
    flex-direction: column;
    gap: 7px;
    z-index: 5;
  }
  .context-exact {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 10px;
    font-size: 11px;
    font-family: "JetBrains Mono", monospace;
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }
  .context-percent {
    color: var(--text-3);
  }
  .context-action {
    width: 100%;
    padding: 6px 8px;
    border-radius: 7px;
    background: var(--bg-4);
    color: var(--text-2);
    font-size: 12px;
  }
  .context-action:hover:not(:disabled) {
    color: var(--text);
    background: color-mix(in srgb, var(--accent) 18%, var(--bg-4));
  }
  .context-action:disabled {
    opacity: 0.45;
    cursor: not-allowed;
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
