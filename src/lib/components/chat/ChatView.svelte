<script lang="ts">
  import { Ghost, ImagePlus as ImagePlusIcon } from "lucide-svelte";
  import { api } from "$lib/api/invoke";
  import { settings } from "$lib/stores/settings";
  import { activeGenerationId, imageMode, incognito } from "$lib/stores/ui";
  import {
    activeChat,
    addVariation,
    appendToMessage,
    deleteMessage,
    ensureChat,
    forkActiveAt,
    persistActive,
    popVariation,
    pushMessage,
    removeMessageById,
    rewindToMessage,
    selectVariation,
    setMessageImageUrl,
    updateMessageContent,
  } from "$lib/stores/chats";
  import type { Attachment, Message } from "$lib/types/chat";
  import { uid } from "$lib/utils/id";
  import { buildMessages, buildParams, buildTools } from "$lib/utils/buildRequest";
  import type { Settings } from "$lib/types/settings";
  import MessageBubble from "./MessageBubble.svelte";
  import InputArea from "./InputArea.svelte";

  let generating = $state(false);
  let generationStreaming = $state(false);
  let isRegenerating = $state(false);
  let isImageGenerating = $state(false);
  let lastError = $state<string | null>(null);
  let activeStreamId = $state<string | null>(null);

  const lastMsgIsUser = $derived(
    !!$activeChat &&
      $activeChat.messages.length > 0 &&
      $activeChat.messages[$activeChat.messages.length - 1].role === "user"
  );

  const canStopCurrent = $derived(generating && activeStreamId !== null);

  $effect(() => {
    activeGenerationId.set(activeStreamId);
  });

  function snapshotSettings(): Settings {
    const s = $settings;
    return {
      ...s,
      params: s.params.map((p) => ({ ...p })),
      reasoning: { ...s.reasoning },
      prompts: s.prompts.map((p) => ({ ...p })),
      custom_colors: { ...s.custom_colors },
    };
  }

  function shouldStream(settings: Settings): boolean {
    return settings.streaming;
  }

  function isImageGenerationModel(settings: Settings): boolean {
    const model = settings.active_model?.toLowerCase() ?? "";
    return model.includes("image");
  }

  function isCancelled(e: unknown): boolean {
    const message = e instanceof Error ? e.message : String(e);
    return message.toLowerCase().includes("generation cancelled");
  }

  async function send(text: string, attachments: Attachment[] = []) {
    if (generating) return;
    if ($imageMode) {
      if (text.trim() || attachments.length) await generateImage(text.trim(), attachments);
      return;
    }
    const requestSettings = snapshotSettings();
    const requestStreaming = shouldStream(requestSettings);
    if (!requestSettings.active_proxy_id || !requestSettings.active_model) {
      lastError = "Сначала выбери прокси и модель";
      return;
    }
    if (isImageGenerationModel(requestSettings)) {
      lastError = "Эта модель генерирует изображения. Переключись в режим генерации изображений.";
      return;
    }
    if (!text && !attachments.length && !lastMsgIsUser) return;
    lastError = null;

    if (text || attachments.length) {
      const title = text || attachments[0]?.name || "Вложение";
      await ensureChat(title);
      pushMessage("user", text, null, attachments);
      await persistActive();
    }

    const messages = buildMessages($activeChat?.messages ?? [], requestSettings);
    const params = buildParams(requestSettings);
    const tools = buildTools(requestSettings);

    const input = {
      proxy_id: requestSettings.active_proxy_id,
      model: requestSettings.active_model,
      messages,
      params,
      tools,
      web_search: requestSettings.web_search,
    };

    generating = true;
    generationStreaming = requestStreaming;
    let streamPlaceholderId: string | null = null;
    try {
      if (requestStreaming) {
        const placeholder = pushMessage("assistant", "", requestSettings.active_model);
        streamPlaceholderId = placeholder.id;
        const streamId = uid();
        activeStreamId = streamId;
        await api.streamCompletion(input, streamId, (ev) => {
          if (ev.type === "chunk") {
            appendToMessage(placeholder.id, ev.content);
          } else if (ev.type === "error") {
            lastError = ev.message;
          }
        });
        const final = $activeChat?.messages.find((m) => m.id === placeholder.id);
        if (final && final.content === "") {
          removeMessageById(placeholder.id);
          streamPlaceholderId = null;
        }
        await persistActive();
      } else {
        const resp = await api.sendCompletion(input);
        const msg = pushMessage("assistant", resp.content, requestSettings.active_model);
        if (resp.image_url) {
          setMessageImageUrl(msg.id, resp.image_url);
        }
        await persistActive();
      }
    } catch (e) {
      if (!isCancelled(e)) {
        lastError = e instanceof Error ? e.message : String(e);
      }
      if (streamPlaceholderId) {
        const final = $activeChat?.messages.find((m) => m.id === streamPlaceholderId);
        if (final && final.content === "") removeMessageById(streamPlaceholderId);
      }
    } finally {
      generating = false;
      generationStreaming = false;
      activeStreamId = null;
      activeGenerationId.set(null);
    }
  }

  async function stop() {
    if (activeStreamId) {
      await api.cancelGeneration(activeStreamId);
    }
  }

  async function generateImage(prompt: string, attachments: Attachment[] = []) {
    if (generating) return;
    const requestSettings = snapshotSettings();
    if (!requestSettings.active_proxy_id || !requestSettings.active_model) {
      lastError = "Сначала выбери прокси и модель";
      return;
    }
    lastError = null;
    generating = true;
    generationStreaming = false;
    isImageGenerating = true;
    const imageId = uid();
    activeStreamId = imageId;
    let placeholderId: string | null = null;

    try {
      const title = prompt || attachments[0]?.name || "Изображение";
      await ensureChat(title);
      pushMessage("user", prompt, null, attachments);
      const placeholder = pushMessage("assistant", "", requestSettings.active_model);
      placeholderId = placeholder.id;
      await persistActive();

      const result = await api.generateImage({
        proxy_id: requestSettings.active_proxy_id,
        model: requestSettings.active_model,
        prompt,
        image_id: imageId,
        attachments,
        params: buildParams(requestSettings),
      });
      setMessageImageUrl(placeholderId, result.url);
      updateMessageContent(placeholderId, prompt);
      await persistActive();
    } catch (e) {
      if (!isCancelled(e)) {
        lastError = e instanceof Error ? e.message : String(e);
      }
      if (placeholderId) {
        removeMessageById(placeholderId);
        await persistActive();
      }
    } finally {
      generating = false;
      generationStreaming = false;
      isImageGenerating = false;
      activeStreamId = null;
      activeGenerationId.set(null);
    }
  }

  async function onEditMessage(id: string, content: string) {
    updateMessageContent(id, content);
    await persistActive();
  }

  async function onDeleteMessage(id: string) {
    deleteMessage(id);
    await persistActive();
  }

  async function onRewindMessage(id: string) {
    rewindToMessage(id);
    await persistActive();
  }

  async function onForkMessage(id: string) {
    if ($incognito) {
      lastError = "Форк недоступен в инкогнито (чат не сохраняется на диск)";
      return;
    }
    await forkActiveAt(id);
  }

  async function onPrevVariation(msg: Message) {
    const idx = (msg.variation_index ?? 0) - 1;
    if (idx < 0) return;
    selectVariation(msg.id, idx);
    await persistActive();
  }

  async function onNextVariation(msg: Message) {
    const total = (msg.variations ?? []).length;
    const idx = (msg.variation_index ?? 0) + 1;
    if (idx >= total) return;
    selectVariation(msg.id, idx);
    await persistActive();
  }

  async function regenerateMessage(msg: Message) {
    if (generating) return;
    const requestSettings = snapshotSettings();
    const requestStreaming = shouldStream(requestSettings);
    if (!requestSettings.active_proxy_id || !requestSettings.active_model) {
      lastError = "Сначала выбери прокси и модель";
      return;
    }
    if (!msg.image_url && isImageGenerationModel(requestSettings)) {
      lastError = "Эта модель генерирует изображения. Переключись в режим генерации изображений.";
      return;
    }
    if (!$activeChat) return;
    lastError = null;

    if (msg.image_url) {
      generating = true;
      generationStreaming = false;
      isImageGenerating = true;
      const imageId = uid();
      activeStreamId = imageId;
      addVariation(msg.id, "", requestSettings.active_model, null);
      try {
        const result = await api.generateImage({
          proxy_id: requestSettings.active_proxy_id,
          model: requestSettings.active_model,
          prompt: msg.content,
          image_id: imageId,
          params: buildParams(requestSettings),
        });
        popVariation(msg.id);
        addVariation(msg.id, msg.content, requestSettings.active_model, result.url);
        await persistActive();
      } catch (e) {
        if (!isCancelled(e)) {
          lastError = e instanceof Error ? e.message : String(e);
        }
        popVariation(msg.id);
      } finally {
        generating = false;
        generationStreaming = false;
        isImageGenerating = false;
        activeStreamId = null;
        activeGenerationId.set(null);
      }
      return;
    }

    const allMessages = $activeChat.messages;
    const idx = allMessages.findIndex((m) => m.id === msg.id);
    if (idx === -1) return;
    const contextSlice = allMessages.slice(0, idx);
    const messages = buildMessages(contextSlice, requestSettings);
    const params = buildParams(requestSettings);
    const tools = buildTools(requestSettings);

    const input = {
      proxy_id: requestSettings.active_proxy_id,
      model: requestSettings.active_model,
      messages,
      params,
      tools,
      web_search: requestSettings.web_search,
    };

    generating = true;
    generationStreaming = requestStreaming;
    isRegenerating = true;
    let acc = "";
    let streamVariationAdded = false;
    try {
      if (requestStreaming) {
        addVariation(msg.id, "", requestSettings.active_model);
        streamVariationAdded = true;
        const streamId = uid();
        activeStreamId = streamId;
        await api.streamCompletion(input, streamId, (ev) => {
          if (ev.type === "chunk") {
            acc += ev.content;
            updateMessageContent(msg.id, acc);
          } else if (ev.type === "error") {
            lastError = ev.message;
          }
        });
        if (acc === "") {
          popVariation(msg.id);
          streamVariationAdded = false;
        } else {
          await persistActive();
        }
      } else {
        addVariation(msg.id, "", requestSettings.active_model);
        try {
          const resp = await api.sendCompletion(input);
          updateMessageContent(msg.id, resp.content);
          if (resp.image_url) {
            setMessageImageUrl(msg.id, resp.image_url);
          }
          await persistActive();
        } catch (inner) {
          popVariation(msg.id);
          throw inner;
        }
      }
    } catch (e) {
      if (!isCancelled(e)) {
        lastError = e instanceof Error ? e.message : String(e);
      }
      if (streamVariationAdded) popVariation(msg.id);
    } finally {
      generating = false;
      generationStreaming = false;
      isRegenerating = false;
      activeStreamId = null;
      activeGenerationId.set(null);
    }
  }
</script>

{#if $incognito}
  <div class="incognito-banner">
    <Ghost size={14} fill="currentColor" />
    Инкогнито — этот чат не сохранится
  </div>
{/if}
{#if $imageMode}
  <div class="imgmode-banner">
    <ImagePlusIcon size={14} />
    Режим генерации изображений
  </div>
{/if}

<div class="chat-area">
  {#if !$activeChat || $activeChat.messages.length === 0}
    <div class="empty-state">
      <div class="empty-card">
        <div class="empty-logo">Scarlet</div>
        <div class="empty-sub">Начните беседу ниже</div>
      </div>
    </div>
  {:else}
    <div class="chat-inner">
      {#each $activeChat.messages as msg, i (msg.id)}
        <MessageBubble
          {msg}
          isLast={i === $activeChat.messages.length - 1}
          onEdit={(c) => onEditMessage(msg.id, c)}
          onDelete={() => onDeleteMessage(msg.id)}
          onRewind={() => onRewindMessage(msg.id)}
          onFork={() => onForkMessage(msg.id)}
          onPrevVariation={() => onPrevVariation(msg)}
          onNextVariation={() => onNextVariation(msg)}
          onRegenerate={() => regenerateMessage(msg)}
        />
      {/each}
      {#if generating && !generationStreaming && !isRegenerating && !isImageGenerating}
        <div class="msg-group">
          <div class="message">
            <div class="role">{$settings.assistant_name || "Scarlet"}</div>
            <div class="typing">
              <span></span><span></span><span></span>
            </div>
          </div>
        </div>
      {/if}
    </div>
  {/if}
</div>

{#if lastError}
  <div class="error" role="alert">
    <span class="error-text">{lastError}</span>
    <button class="error-close" onclick={() => (lastError = null)} title="Закрыть">×</button>
  </div>
{/if}

<InputArea onSend={send} onStop={stop} busy={generating} canStop={canStopCurrent} canResend={lastMsgIsUser} />

<style>
  .chat-area {
    flex: 1;
    overflow-y: auto;
    padding: 20px 0 36px;
    display: flex;
    flex-direction: column;
    -webkit-mask-image: linear-gradient(
      to bottom,
      black 0%,
      black calc(100% - 28px),
      transparent 100%
    );
    mask-image: linear-gradient(
      to bottom,
      black 0%,
      black calc(100% - 28px),
      transparent 100%
    );
  }
  .chat-inner {
    width: 100%;
    padding: 0 24px;
    display: flex;
    flex-direction: column;
    gap: 0;
  }

  .empty-state {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: flex-start;
    gap: 10px;
    color: var(--text-3);
    padding: clamp(72px, 16vh, 150px) 20px 60px;
  }
  .empty-card {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 10px;
    padding: 28px 36px;
    background: var(--bg);
    border-radius: 18px;
    border: 1px solid var(--border);
  }
  .empty-logo {
    font-size: 40px;
    font-weight: 700;
    color: var(--accent);
    letter-spacing: -0.02em;
  }
  .empty-sub {
    font-size: 14px;
  }

  .msg-group {
    display: flex;
    flex-direction: column;
    padding: 4px 0 28px;
    align-items: flex-start;
    width: 100%;
  }
  .message {
    max-width: min(900px, 94%);
    padding: 10px 14px;
    border-radius: 14px;
    background: var(--bg-3);
    border-bottom-left-radius: 4px;
  }
  .role {
    font-size: 10px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--text-3);
    margin-bottom: 4px;
  }
  .typing {
    display: flex;
    gap: 5px;
    align-items: center;
    padding: 4px 2px;
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

  .incognito-banner {
    background: oklch(14% 0.025 280);
    border-bottom: 1px solid oklch(26% 0.04 280);
    padding: 7px 18px;
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
    color: oklch(62% 0.08 280);
  }
  .imgmode-banner {
    background: color-mix(in srgb, var(--accent) 10%, var(--bg));
    border-bottom: 1px solid color-mix(in srgb, var(--accent) 25%, var(--border));
    padding: 7px 18px;
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
    color: var(--accent);
  }

  .error {
    margin: 0 24px 8px;
    padding: 8px 12px;
    background: oklch(20% 0.05 25);
    border: 1px solid var(--danger);
    color: oklch(80% 0.05 25);
    border-radius: 8px;
    font-size: 12px;
    display: flex;
    align-items: flex-start;
    gap: 8px;
  }
  .error-text {
    flex: 1;
    white-space: pre-wrap;
    word-break: break-word;
  }
  .error-close {
    flex-shrink: 0;
    background: none;
    border: none;
    color: oklch(60% 0.05 25);
    cursor: pointer;
    font-size: 16px;
    line-height: 1;
    padding: 0;
    margin-top: -1px;
  }
  .error-close:hover {
    color: oklch(80% 0.05 25);
  }
</style>
