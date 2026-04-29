<script lang="ts">
  import {
    Copy,
    Pencil,
    Trash2,
    Scissors,
    GitFork,
    Check,
    ChevronLeft,
    ChevronRight,
    Download,
    FileText,
    Bookmark,
  } from "lucide-svelte";
  import type { Attachment, Message } from "$lib/types/chat";
  import { tr } from "$lib/i18n";
  import { settings } from "$lib/stores/settings";
  import { sidebarOpen } from "$lib/stores/ui";
  import Markdown from "$lib/components/common/Markdown.svelte";
  import { api } from "$lib/api/invoke";
  import { estimateStoredMessageTokens } from "$lib/utils/buildRequest";

  let {
    msg,
    onEdit,
    onDelete,
    onDeleteGroup,
    onRewind,
    onFork,
    onPrevBranch,
    onNextBranch,
    onRegenerate,
    onSendEdit,
    onToggleBookmark,
    branchIndex = 0,
    branchCount = 1,
    branchLocked = false,
    highlighted = false,
  }: {
    msg: Message;
    onEdit?: (content: string) => void;
    onDelete?: () => void;
    onDeleteGroup?: () => void;
    onRewind?: () => void;
    onFork?: () => void;
    onPrevBranch?: () => void;
    onNextBranch?: () => void;
    onRegenerate?: () => void;
    onSendEdit?: (content: string) => void;
    onToggleBookmark?: () => void;
    branchIndex?: number;
    branchCount?: number;
    branchLocked?: boolean;
    highlighted?: boolean;
  } = $props();

  let hovered = $state(false);
  let editing = $state(false);
  let editVal = $state("");
  let copied = $state(false);
  let deleteChoiceOpen = $state(false);
  let editAreaEl: HTMLTextAreaElement | undefined = $state();
  let touchStartX: number | null = null;
  let viewerOpen = $state(false);
  let viewerSrc = $state("");
  let viewerAlt = $state("");
  let viewerScale = $state(1);
  let viewerX = $state(0);
  let viewerY = $state(0);
  let viewerDragging = $state(false);
  let viewerDragX = 0;
  let viewerDragY = 0;
  let viewerStartX = 0;
  let viewerStartY = 0;

  $effect(() => {
    if (editing && editAreaEl) {
      editAreaEl.focus();
      autoResize();
    }
  });

  $effect(() => {
    if (!hovered) deleteChoiceOpen = false;
  });

  function autoResize() {
    if (!editAreaEl) return;
    editAreaEl.style.height = "auto";
    editAreaEl.style.height = Math.min(editAreaEl.scrollHeight, 480) + "px";
  }

  const isUser = $derived(msg.role === "user");
  const isAssistant = $derived(msg.role === "assistant");
  const isEmptyAssistant = $derived(isAssistant && msg.content === "");

  const branchIdx = $derived(Math.max(0, branchIndex));
  const branchTotal = $derived(Math.max(1, branchCount));
  const atFirstBranch = $derived(branchIdx === 0);
  const atLastBranch = $derived(branchIdx >= branchTotal - 1);
  const displayModel = $derived(msg.model ?? null);
  const displayTime = $derived(msg.created_at);
  const displayImageUrl = $derived(msg.image_url ?? null);
  const tokenCount = $derived(estimateStoredMessageTokens(msg));

  function fmtTime(iso: string): string {
    try {
      const d = new Date(iso);
      const today = new Date();
      const sameDay =
        d.getFullYear() === today.getFullYear() &&
        d.getMonth() === today.getMonth() &&
        d.getDate() === today.getDate();
      const hh = String(d.getHours()).padStart(2, "0");
      const mm = String(d.getMinutes()).padStart(2, "0");
      if (sameDay) return `${hh}:${mm}`;
      return `${d.toLocaleDateString()} ${hh}:${mm}`;
    } catch {
      return "";
    }
  }

  async function downloadImage() {
    if (!displayImageUrl) return;
    let ext = "png";
    const m = displayImageUrl.match(/^data:image\/([a-zA-Z0-9+]+);/);
    if (m) ext = m[1].replace("jpeg", "jpg");
    await api.saveImage(displayImageUrl, `scarlet-${Date.now()}.${ext}`, $tr("message.saveImageTitle"));
  }

  async function downloadAttachmentImage(att: Attachment) {
    const dataUrl = `data:${att.mimeType};base64,${att.data}`;
    const fallbackExt = att.mimeType.split("/")[1]?.replace("jpeg", "jpg") || "png";
    const name = /\.[A-Za-z0-9]+$/.test(att.name) ? att.name : `${att.name || "scarlet-image"}.${fallbackExt}`;
    await api.saveImage(dataUrl, name, $tr("message.saveImageTitle"));
  }

  function copy() {
    navigator.clipboard.writeText(msg.content).catch(() => {});
    copied = true;
    setTimeout(() => (copied = false), 1500);
  }

  function startEdit() {
    editVal = msg.content;
    editing = true;
  }

  function saveEdit() {
    if (editVal.trim()) onEdit?.(editVal.trim());
    editing = false;
  }

  function sendEdit() {
    if (editVal.trim()) onSendEdit?.(editVal.trim());
    editing = false;
  }

  function onEditKey(e: KeyboardEvent) {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      saveEdit();
    } else if (e.key === "Escape") {
      editing = false;
    }
  }

  function onTouchStart(e: TouchEvent) {
    if (branchLocked) return;
    touchStartX = e.touches[0].clientX;
  }
  function onTouchEnd(e: TouchEvent) {
    if (branchLocked || touchStartX === null) return;
    const diff = e.changedTouches[0].clientX - touchStartX;
    if (diff < -50) {
      if (isAssistant && atLastBranch) onRegenerate?.();
      else if (!atLastBranch) onNextBranch?.();
    } else if (diff > 50 && !atFirstBranch) {
      onPrevBranch?.();
    }
    touchStartX = null;
  }

  function clickNext() {
    if (branchLocked) return;
    if (isAssistant && atLastBranch) onRegenerate?.();
    else onNextBranch?.();
  }

  function clickDelete() {
    if (branchTotal > 1) {
      deleteChoiceOpen = !deleteChoiceOpen;
      return;
    }
    onDelete?.();
  }

  function deleteSwipe() {
    deleteChoiceOpen = false;
    onDelete?.();
  }

  function deleteWholeMessage() {
    deleteChoiceOpen = false;
    onDeleteGroup?.();
  }

  function openImageViewer(src: string, alt: string) {
    viewerSrc = src;
    viewerAlt = alt;
    viewerScale = 1;
    viewerX = 0;
    viewerY = 0;
    viewerDragging = false;
    viewerOpen = true;
  }

  function closeImageViewer() {
    viewerOpen = false;
    viewerDragging = false;
  }

  function onViewerWheel(e: WheelEvent) {
    e.preventDefault();
    const oldScale = viewerScale;
    const nextScale = Math.min(8, Math.max(0.35, oldScale * (e.deltaY < 0 ? 1.12 : 1 / 1.12)));
    const backdrop = (e.currentTarget as HTMLElement).closest(".image-viewer-backdrop");
    const rect = backdrop?.getBoundingClientRect() ?? (e.currentTarget as HTMLElement).getBoundingClientRect();
    const pointerX = e.clientX - rect.left - rect.width / 2;
    const pointerY = e.clientY - rect.top - rect.height / 2;
    const ratio = nextScale / oldScale;
    viewerX = pointerX - (pointerX - viewerX) * ratio;
    viewerY = pointerY - (pointerY - viewerY) * ratio;
    viewerScale = nextScale;
  }

  function onViewerPointerDown(e: PointerEvent) {
    e.preventDefault();
    viewerDragging = true;
    viewerDragX = e.clientX;
    viewerDragY = e.clientY;
    viewerStartX = viewerX;
    viewerStartY = viewerY;
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
  }

  function onViewerPointerMove(e: PointerEvent) {
    if (!viewerDragging) return;
    viewerX = viewerStartX + e.clientX - viewerDragX;
    viewerY = viewerStartY + e.clientY - viewerDragY;
  }

  function onViewerPointerUp(e: PointerEvent) {
    viewerDragging = false;
    (e.currentTarget as HTMLElement).releasePointerCapture(e.pointerId);
  }

  function onViewerBackdropClick(e: MouseEvent) {
    if ((e.target as HTMLElement).closest(".image-viewer-frame")) return;
    closeImageViewer();
  }

  function onViewerKey(e: KeyboardEvent) {
    if (e.key === "Escape") closeImageViewer();
  }
</script>

<div
  class="msg-group"
  class:user={isUser}
  class:assistant={isAssistant}
  onmouseenter={() => (hovered = true)}
  onmouseleave={() => (hovered = false)}
  ontouchstart={onTouchStart}
  ontouchend={onTouchEnd}
  role="group"
>
  <div class="message" class:user={isUser} class:assistant={isAssistant} class:editing class:highlighted>
    <div class="role-row">
      <span class="role">{isUser ? $settings.user_name || $tr("message.userFallback") : $settings.assistant_name || "Scarlet"}</span>
      {#if isAssistant && displayModel}
        <span class="model-tag" title={displayModel}>{displayModel}</span>
      {/if}
      {#if isAssistant && $settings.show_token_counts && !isEmptyAssistant}
        <span class="token-tag" title={`${tokenCount} tokens`}>{tokenCount} tok</span>
      {/if}
      <span class="time">{fmtTime(displayTime)}</span>
    </div>

    {#if editing}
      <div class="edit">
        <textarea
          bind:value={editVal}
          onkeydown={onEditKey}
          oninput={autoResize}
          bind:this={editAreaEl}
        ></textarea>
        <div class="edit-actions">
          {#if isUser && onSendEdit}
            <button class="btn-sm send" onclick={sendEdit}>{$tr("message.sendEdit")}</button>
          {/if}
          <button class="btn-sm save" onclick={saveEdit}>{$tr("common.save")}</button>
          <button class="btn-sm" onclick={() => (editing = false)}>{$tr("common.cancel")}</button>
        </div>
      </div>
    {:else if isEmptyAssistant}
      <div class="typing">
        <span></span><span></span><span></span>
      </div>
    {:else if displayImageUrl}
      <div class="img-wrap">
        <button
          type="button"
          class="img-open"
          onclick={() => openImageViewer(displayImageUrl, msg.content || $tr("message.generatedImage"))}
          aria-label={msg.content || $tr("message.generatedImage")}
        >
          <img
            src={displayImageUrl}
            alt={msg.content || $tr("message.generatedImage")}
            class="generated-img"
            loading="lazy"
            draggable="false"
          />
        </button>
        <button class="img-download" title={$tr("message.download")} onclick={(e) => {
          e.stopPropagation();
          downloadImage();
        }}>
          <Download size={14} />
        </button>
      </div>
      {#if msg.content}
        <p class="img-caption">{msg.content}</p>
      {/if}
    {:else if isUser}
      {#if msg.attachments && msg.attachments.length > 0}
        <div class="att-preview">
          {#each msg.attachments as att (att.id)}
            {#if att.mimeType.startsWith("image/")}
              <div class="att-img-wrap">
                <button
                  type="button"
                  class="att-img-open"
                  onclick={() => openImageViewer(`data:${att.mimeType};base64,${att.data}`, att.name)}
                  aria-label={att.name}
                >
                  <img
                    src={`data:${att.mimeType};base64,${att.data}`}
                    alt={att.name}
                    class="att-img"
                    draggable="false"
                  />
                </button>
                <button class="img-download" title={$tr("message.download")} onclick={(e) => {
                  e.stopPropagation();
                  downloadAttachmentImage(att);
                }}>
                  <Download size={14} />
                </button>
              </div>
            {:else}
              <div class="att-file">
                <FileText size={14} />
                <span>{att.name}</span>
              </div>
            {/if}
          {/each}
        </div>
      {/if}
      {#if msg.content}
        <div class="content">{msg.content}</div>
      {/if}
    {:else}
      <Markdown content={msg.content} />
    {/if}

    {#if !editing && !isEmptyAssistant}
      <div class="toolbar" class:visible={hovered}>
        <div class="toolbar-left">
          <button class="tb-btn" title={$tr("message.copy")} onclick={copy}>
            {#if copied}
              <Check size={13} color="var(--accent)" />
            {:else}
              <Copy size={13} />
            {/if}
          </button>
          <button
            class="tb-btn"
            class:active={msg.bookmarked}
            title={$tr("message.bookmark")}
            onclick={() => onToggleBookmark?.()}
          >
            <Bookmark size={13} fill={msg.bookmarked ? "currentColor" : "none"} />
          </button>
          <button class="tb-btn" title={$tr("common.edit")} onclick={startEdit}>
            <Pencil size={13} />
          </button>
          <div class="delete-wrap">
            {#if deleteChoiceOpen}
              <div class="delete-choice">
                <button onclick={deleteWholeMessage}>{$tr("message.deleteMessage")}</button>
                <button onclick={deleteSwipe}>{$tr("message.deleteSwipe")}</button>
              </div>
            {/if}
            <button class="tb-btn" title={$tr("common.delete")} onclick={clickDelete}>
              <Trash2 size={13} />
            </button>
          </div>
          <button class="tb-btn" title={$tr("message.rewind")} onclick={() => onRewind?.()}>
            <Scissors size={13} />
          </button>
          <button class="tb-btn" title={$tr("message.fork")} onclick={() => onFork?.()}>
            <GitFork size={13} />
          </button>
        </div>

        {#if branchTotal > 1 || isAssistant}
          <div class="toolbar-right">
            {#if branchTotal > 1}
              <button
                class="tb-btn"
                disabled={branchLocked || atFirstBranch}
                title={$tr("message.prevVariant")}
                onclick={() => onPrevBranch?.()}
              >
                <ChevronLeft size={13} />
              </button>
              <span class="var-count">{branchIdx + 1}/{branchTotal}</span>
            {/if}
            <button
              class="tb-btn"
              disabled={branchLocked || (!isAssistant && atLastBranch)}
              title={isAssistant && atLastBranch ? $tr("message.regenerateVariant") : $tr("message.nextVariant")}
              onclick={clickNext}
            >
              <ChevronRight size={13} />
            </button>
          </div>
        {/if}
      </div>
    {/if}
  </div>
</div>

{#if viewerOpen}
  <div
    class="image-viewer-backdrop"
    class:sidebar-visible={$sidebarOpen}
    onclick={onViewerBackdropClick}
    onkeydown={onViewerKey}
    role="dialog"
    aria-modal="true"
    tabindex="-1"
  >
    <div
      class="image-viewer-stage"
      class:dragging={viewerDragging}
      role="presentation"
    >
      <div
        class="image-viewer-frame"
        style={`transform: translate3d(${viewerX}px, ${viewerY}px, 0) scale(${viewerScale});`}
        onwheel={onViewerWheel}
        onpointerdown={onViewerPointerDown}
        onpointermove={onViewerPointerMove}
        onpointerup={onViewerPointerUp}
        onpointercancel={onViewerPointerUp}
        role="presentation"
      >
        <img
          src={viewerSrc}
          alt={viewerAlt}
          class="image-viewer-img"
          draggable="false"
        />
      </div>
    </div>
  </div>
{/if}

<style>
  .msg-group {
    display: flex;
    flex-direction: column;
    padding: 4px 0 40px;
    position: relative;
    width: 100%;
  }
  .msg-group.user {
    align-items: flex-end;
  }
  .msg-group.assistant {
    align-items: flex-start;
  }

  .message {
    position: relative;
    max-width: min(1000px, 96%);
    padding: 12px 16px;
    border-radius: 14px;
    background: var(--bg-3);
    animation: msgIn 0.18s ease;
    transition: box-shadow 0.18s ease, background 0.18s ease;
  }
  .message.highlighted {
    box-shadow: 0 0 0 1px color-mix(in srgb, var(--highlight) 58%, transparent),
      0 0 22px color-mix(in srgb, var(--highlight) 22%, transparent);
  }
  .message.user {
    background: var(--user-msg);
    color: var(--user-text);
    border-bottom-right-radius: 4px;
  }
  .message.assistant {
    border-bottom-left-radius: 4px;
  }
  .message.editing {
    width: min(1000px, 96%);
    max-width: min(1000px, 96%);
  }
  @keyframes msgIn {
    from {
      opacity: 0;
      transform: translateY(6px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  .role-row {
    display: flex;
    align-items: baseline;
    gap: 10px;
    margin-bottom: 6px;
  }
  .role {
    font-size: 12px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-2);
  }
  .message.user .role {
    color: rgba(255, 255, 255, 0.85);
  }
  .model-tag {
    font-size: 11px;
    color: var(--text-2);
    font-family: "JetBrains Mono", monospace;
    padding: 2px 8px;
    background: var(--bg-4);
    border-radius: 5px;
    max-width: 240px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .token-tag {
    font-size: 11px;
    color: var(--text-3);
    font-family: "JetBrains Mono", monospace;
    padding: 2px 7px;
    background: color-mix(in srgb, var(--bg-4) 72%, transparent);
    border-radius: 5px;
    white-space: nowrap;
  }
  .message.user .model-tag {
    background: rgba(0, 0, 0, 0.25);
    color: rgba(255, 255, 255, 0.8);
  }
  .time {
    margin-left: auto;
    font-size: 11px;
    color: var(--text-2);
    font-variant-numeric: tabular-nums;
  }
  .message.user .time {
    color: rgba(255, 255, 255, 0.7);
  }
  .content {
    white-space: pre-wrap;
    word-break: break-word;
    line-height: 1.65;
  }

  .att-preview {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 6px;
    margin-bottom: 6px;
  }
  .att-img-open,
  .img-open {
    display: block;
    max-width: 100%;
    padding: 0;
    border: 0;
    border-radius: 8px;
    background: transparent;
    color: inherit;
    text-align: inherit;
  }
  .att-img {
    max-width: 260px;
    max-height: 200px;
    border-radius: 8px;
    object-fit: cover;
    display: block;
    cursor: zoom-in;
  }
  .att-img-wrap {
    position: relative;
    display: inline-block;
    max-width: 100%;
  }
  .att-file {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    background: rgba(0, 0, 0, 0.2);
    border-radius: 6px;
    padding: 4px 8px;
    max-width: 220px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .img-wrap {
    position: relative;
    display: inline-block;
    max-width: 100%;
  }
  .img-open {
    display: inline-block;
  }
  .generated-img {
    max-width: 100%;
    border-radius: 8px;
    display: block;
    cursor: zoom-in;
  }
  .img-download {
    position: absolute;
    top: 8px;
    right: 8px;
    background: oklch(15% 0 0 / 0.6);
    color: var(--text-1);
    border: 1px solid oklch(100% 0 0 / 0.12);
    border-radius: 8px;
    width: 30px;
    height: 30px;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    opacity: 0;
    transition: opacity 0.15s, background 0.15s;
    backdrop-filter: blur(6px);
  }
  .img-wrap:hover .img-download,
  .att-img-wrap:hover .img-download {
    opacity: 1;
  }
  .img-download:hover {
    background: oklch(20% 0 0 / 0.8);
  }
  .img-caption {
    font-size: 12px;
    color: var(--text-3);
    margin-top: 6px;
    font-style: italic;
  }

  .image-viewer-backdrop {
    position: fixed;
    inset: 0;
    z-index: 10000;
    overflow: hidden;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 28px;
    background: oklch(0% 0 0 / 0.78);
    backdrop-filter: blur(8px);
  }
  @media (min-width: 768px) {
    .image-viewer-backdrop.sidebar-visible {
      left: 256px;
    }
  }
  .image-viewer-stage {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    touch-action: none;
    user-select: none;
  }
  .image-viewer-frame {
    display: flex;
    align-items: center;
    justify-content: center;
    max-width: min(92%, 1400px);
    max-height: 92%;
    padding: 8px;
    border: 1px solid color-mix(in srgb, var(--border) 68%, white 18%);
    border-radius: 10px;
    background: color-mix(in srgb, var(--bg) 36%, transparent);
    box-shadow:
      0 0 0 1px oklch(100% 0 0 / 0.08),
      0 24px 80px oklch(0% 0 0 / 0.55);
    cursor: grab;
    transform-origin: center center;
    will-change: transform;
    touch-action: none;
  }
  .image-viewer-stage.dragging .image-viewer-frame {
    cursor: grabbing;
  }
  .image-viewer-img {
    max-width: 100%;
    max-height: calc((100vh / var(--app-scale, 1)) * 0.86);
    width: auto;
    height: auto;
    display: block;
    border-radius: 6px;
    pointer-events: none;
  }

  .typing {
    display: flex;
    gap: 5px;
    align-items: center;
    padding: 6px 2px 4px;
  }
  .typing span {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--text-3);
    animation: bounce 1.2s infinite;
  }
  .typing span:nth-child(2) {
    animation-delay: 0.18s;
  }
  .typing span:nth-child(3) {
    animation-delay: 0.36s;
  }
  @keyframes bounce {
    0%,
    80%,
    100% {
      transform: translateY(0);
      opacity: 0.4;
    }
    40% {
      transform: translateY(-6px);
      opacity: 1;
    }
  }

  .edit {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .edit textarea {
    background: var(--bg-4);
    border-radius: 8px;
    padding: 10px 12px;
    border: 1px solid var(--border);
    font-size: 14px;
    min-height: 160px;
    max-height: 480px;
    width: 100%;
    color: var(--text);
    line-height: 1.55;
    overflow-y: auto;
    resize: vertical;
  }
  .edit textarea:focus {
    border-color: var(--accent-d);
  }
  .message.user .edit textarea {
    background: rgba(0, 0, 0, 0.25);
    border-color: rgba(255, 255, 255, 0.15);
    color: white;
  }
  .edit-actions {
    display: flex;
    gap: 6px;
  }
  .btn-sm {
    padding: 5px 12px;
    border-radius: 7px;
    font-size: 12px;
    font-weight: 500;
    background: var(--bg-4);
    color: var(--text-2);
    transition: background 0.12s;
  }
  .btn-sm:hover {
    background: var(--border);
    color: var(--text);
  }
  .btn-sm.save {
    background: var(--accent-d);
    color: white;
    border: 1px solid color-mix(in srgb, var(--accent-d) 70%, white);
  }
  .btn-sm.save:hover {
    background: color-mix(in srgb, var(--accent-d) 82%, white);
    color: white;
  }
  .btn-sm.send {
    background: var(--accent-d);
    color: white;
    border: 1px solid color-mix(in srgb, var(--accent-d) 70%, white);
  }
  .btn-sm.send:hover {
    background: color-mix(in srgb, var(--accent-d) 82%, white);
    color: white;
  }
  .message.user .btn-sm {
    background: rgba(0, 0, 0, 0.22);
    color: rgba(255, 255, 255, 0.85);
    border: 1px solid rgba(255, 255, 255, 0.14);
  }
  .message.user .btn-sm:hover {
    background: rgba(0, 0, 0, 0.34);
    color: white;
  }
  .message.user .btn-sm.save {
    background: var(--accent-d);
    color: white;
    border-color: color-mix(in srgb, var(--accent-d) 70%, white);
  }
  .message.user .btn-sm.send {
    background: var(--accent-d);
    color: white;
    border-color: color-mix(in srgb, var(--accent-d) 70%, white);
  }
  .message.user .btn-sm.save:hover,
  .message.user .btn-sm.send:hover {
    background: color-mix(in srgb, var(--accent-d) 82%, white);
    color: white;
  }

  /* Toolbar внутри bubble — прибит ниже под её нижним краем,
     поэтому стрелки вариантов всегда у правого края bubble. */
  .toolbar {
    position: absolute;
    bottom: -34px;
    left: 0;
    right: 0;
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0;
    opacity: 0;
    pointer-events: none;
    transition: opacity 0.15s;
  }
  .toolbar.visible {
    opacity: 1;
    pointer-events: all;
  }
  /* User-сообщение: только левый toolbar, его прижимаем к правому краю */
  .msg-group.user .toolbar {
    justify-content: flex-end;
  }

  .toolbar-left,
  .toolbar-right {
    display: flex;
    align-items: center;
    gap: 1px;
    background: var(--bg-3);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 3px;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.25);
  }
  .delete-wrap {
    position: relative;
    display: flex;
  }
  .delete-choice {
    position: absolute;
    left: 50%;
    bottom: calc(100% + 7px);
    transform: translateX(-50%);
    display: flex;
    gap: 4px;
    padding: 4px;
    border-radius: 8px;
    background: var(--bg-3);
    border: 1px solid var(--border);
    box-shadow: var(--shadow);
    z-index: 6;
  }
  .delete-choice::after {
    content: "";
    position: absolute;
    left: 50%;
    bottom: -5px;
    width: 8px;
    height: 8px;
    background: var(--bg-3);
    border-right: 1px solid var(--border);
    border-bottom: 1px solid var(--border);
    transform: translateX(-50%) rotate(45deg);
  }
  .delete-choice button {
    position: relative;
    z-index: 1;
    min-height: 26px;
    padding: 3px 8px;
    border-radius: 6px;
    color: var(--text-2);
    font-size: 12px;
    white-space: nowrap;
  }
  .delete-choice button:hover {
    background: color-mix(in srgb, var(--danger) 14%, var(--bg-4));
    color: var(--text);
  }
  .tb-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 26px;
    height: 26px;
    border-radius: 5px;
    color: var(--text-3);
    transition: background 0.1s, color 0.1s;
  }
  .tb-btn:hover:not(:disabled) {
    background: var(--bg-4);
    color: var(--text);
  }
  .tb-btn.active {
    color: var(--accent);
  }
  .tb-btn:disabled {
    opacity: 0.3;
    cursor: default;
  }
  .var-count {
    font-size: 11px;
    color: var(--text-3);
    padding: 0 4px;
    min-width: 32px;
    text-align: center;
    font-variant-numeric: tabular-nums;
  }
</style>
