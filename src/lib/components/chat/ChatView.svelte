<script lang="ts">
  import { Ghost, ImagePlus as ImagePlusIcon } from "lucide-svelte";
  import { api } from "$lib/api/invoke";
  import { tr } from "$lib/i18n";
  import { settings } from "$lib/stores/settings";
  import { activeGenerationId, imageMode, incognito } from "$lib/stores/ui";
  import {
    activeChat,
    addVariation,
    appendToMessage,
    deleteMessage,
    deleteSummary,
    ensureChat,
    forkActiveAt,
    persistActive,
    popVariation,
    pushMessage,
    removeMessageById,
    rewindToMessage,
    selectVariation,
    setSummary,
    setMessageImageUrl,
    updateSummaryContent,
    updateMessageContent,
  } from "$lib/stores/chats";
  import type { Attachment, Chat, ChatMessage, Message } from "$lib/types/chat";
  import { uid } from "$lib/utils/id";
  import { buildMessages, buildParams, buildTools, estimateContextTokens, transcriptFromMessages } from "$lib/utils/buildRequest";
  import { DEFAULT_SUMMARIZE_PROMPT, type Settings } from "$lib/types/settings";
  import MessageBubble from "./MessageBubble.svelte";
  import InputArea from "./InputArea.svelte";
  import SummaryBoundary from "./SummaryBoundary.svelte";

  let generating = $state(false);
  let generationStreaming = $state(false);
  let isRegenerating = $state(false);
  let isImageGenerating = $state(false);
  let isSummarizing = $state(false);
  let lastError = $state<string | null>(null);
  let activeStreamId = $state<string | null>(null);
  let summarizeStopRequested = false;

  const lastMsgIsUser = $derived(
    !!$activeChat &&
      $activeChat.messages.length > 0 &&
      $activeChat.messages[$activeChat.messages.length - 1].role === "user"
  );

  const canStopCurrent = $derived(generating && activeStreamId !== null);
  const contextUsed = $derived(estimateContextTokens($activeChat?.messages ?? [], $settings, $activeChat?.summary ?? null));
  const contextWindow = $derived(Math.max(0, Number($settings.context_window ?? 0)));
  const canSummarize = $derived(!!$activeChat && $activeChat.messages.length > 0 && !generating);

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
      utilities: { ...s.utilities },
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

  const TRUNCATED_MARKER = "\n\n[message truncated]";

  function maxAssistantChars(settings: Settings): number {
    const raw = Number(settings.max_message_size ?? 0);
    return Number.isFinite(raw) && raw > 0 ? Math.floor(raw) : 0;
  }

  function limitAssistantMessage(content: string, settings: Settings): string {
    const limit = maxAssistantChars(settings);
    if (limit <= 0 || content.length <= limit) return content;
    if (limit <= TRUNCATED_MARKER.length) return content.slice(0, limit);
    return content.slice(0, limit - TRUNCATED_MARKER.length) + TRUNCATED_MARKER;
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
      lastError = $tr("chat.pickProxyAndModel");
      return;
    }
    if (isImageGenerationModel(requestSettings)) {
      lastError = $tr("chat.imageModelInChatError");
      return;
    }
    if (!text && !attachments.length && !lastMsgIsUser) return;
    lastError = null;

    if (!(await maybeAutoSummarize(requestSettings))) {
      return;
    }

    if (text || attachments.length) {
      const title = text || attachments[0]?.name || $tr("chat.attachment");
      await ensureChat(title);
      pushMessage("user", text, null, attachments);
      await persistActive();
    }

    const messages = buildMessages($activeChat?.messages ?? [], requestSettings, $activeChat?.summary ?? null);
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
        let streamedContent = "";
        let streamLimitReached = false;
        await api.streamCompletion(input, streamId, (ev) => {
          if (ev.type === "chunk") {
            if (streamLimitReached) return;
            const next = streamedContent + ev.content;
            const limited = limitAssistantMessage(next, requestSettings);
            if (limited !== next) {
              streamedContent = limited;
              streamLimitReached = true;
              updateMessageContent(placeholder.id, limited);
              void api.cancelGeneration(streamId);
            } else {
              streamedContent = next;
              appendToMessage(placeholder.id, ev.content);
            }
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
        const msg = pushMessage(
          "assistant",
          limitAssistantMessage(resp.content, requestSettings),
          requestSettings.active_model
        );
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
      if (isSummarizing) summarizeStopRequested = true;
      await api.cancelGeneration(activeStreamId);
    }
  }

  function resolveSummarizePrompt(requestSettings: Settings): string {
    const mappedId = requestSettings.utilities.summarize_prompt_id;
    const mapped = mappedId ? requestSettings.prompts.find((p) => p.id === mappedId) : null;
    return (
      mapped?.content.trim() ||
      requestSettings.utilities.summarize_default_prompt.trim() ||
      DEFAULT_SUMMARIZE_PROMPT
    );
  }

  function summarizeTranscript(chat: Chat, boundaryId?: string): string {
    const boundaryIdx = boundaryId
      ? chat.messages.findIndex((m) => m.id === boundaryId)
      : chat.messages.length - 1;
    const endIdx = boundaryIdx === -1 ? chat.messages.length - 1 : boundaryIdx;
    const targetMessages = chat.messages.slice(0, endIdx + 1);
    const existing = chat.summary;
    if (existing?.content.trim()) {
      const idx = chat.messages.findIndex((m) => m.id === existing.after_message_id);
      if (idx !== -1 && idx <= endIdx) {
        return [
          "Existing summary of earlier conversation:",
          existing.content.trim(),
          "",
          "Conversation after that summary:",
          transcriptFromMessages(chat.messages.slice(idx + 1, endIdx + 1)),
        ].join("\n");
      }
    }
    return transcriptFromMessages(targetMessages);
  }

  async function runSummarize(chat: Chat, requestSettings: Settings, boundary: Message): Promise<boolean> {
    const prompt = resolveSummarizePrompt(requestSettings);
    const messages: ChatMessage[] = [
      { role: "system", content: prompt },
      { role: "user", content: `Conversation to compress:\n\n${summarizeTranscript(chat, boundary.id)}` },
    ];

    lastError = null;
    generating = true;
    generationStreaming = false;
    isSummarizing = true;
    summarizeStopRequested = false;
    const streamId = uid();
    activeStreamId = streamId;
    let content = "";
    try {
      await api.streamCompletion(
        {
          proxy_id: requestSettings.active_proxy_id!,
          model: requestSettings.active_model!,
          messages,
          params: buildParams(requestSettings),
          tools: [],
          web_search: false,
        },
        streamId,
        (ev) => {
          if (ev.type === "chunk") {
            content += ev.content;
          } else if (ev.type === "error" && !isCancelled(ev.message)) {
            lastError = ev.message;
          }
        }
      );

      if (summarizeStopRequested || $activeChat?.id !== chat.id) return false;

      const summary = content.trim();
      if (!summary) {
        lastError = $tr("chat.summarizeFailed");
        return false;
      }
      const now = new Date().toISOString();
      setSummary({
        id: uid(),
        content: summary,
        prompt,
        after_message_id: boundary.id,
        model: requestSettings.active_model,
        created_at: now,
        updated_at: now,
      });
      await persistActive();
      return true;
    } catch (e) {
      if (!isCancelled(e)) {
        lastError = e instanceof Error ? e.message : String(e);
      }
      return false;
    } finally {
      generating = false;
      generationStreaming = false;
      isSummarizing = false;
      summarizeStopRequested = false;
      activeStreamId = null;
      activeGenerationId.set(null);
    }
  }

  function contextLimitExceeded(chat: Chat, requestSettings: Settings): boolean {
    const limit = Math.max(0, Number(requestSettings.context_window ?? 0));
    return limit > 0 && estimateContextTokens(chat.messages, requestSettings, chat.summary ?? null) > limit;
  }

  function autoSummarizeBoundary(chat: Chat): Message | null {
    if (chat.messages.length <= 1) return null;
    const last = chat.messages[chat.messages.length - 1];
    const boundaryIndex = last.role === "user" ? chat.messages.length - 2 : chat.messages.length - 1;
    if (boundaryIndex < 0) return null;
    const boundary = chat.messages[boundaryIndex];
    if (chat.summary?.after_message_id === boundary.id) return null;
    return boundary;
  }

  async function maybeAutoSummarize(requestSettings: Settings): Promise<boolean> {
    if (!requestSettings.utilities.auto_summarize) return true;
    const chat = $activeChat;
    if (!chat || !contextLimitExceeded(chat, requestSettings)) return true;
    const boundary = autoSummarizeBoundary(chat);
    if (!boundary) return true;
    return runSummarize(chat, requestSettings, boundary);
  }

  async function summarizeChat() {
    if (generating || !$activeChat || $activeChat.messages.length === 0) return;
    const chat = $activeChat;
    const requestSettings = snapshotSettings();
    if (!requestSettings.active_proxy_id || !requestSettings.active_model) {
      lastError = $tr("chat.pickProxyAndModel");
      return;
    }

    const boundary = chat.messages[chat.messages.length - 1];
    await runSummarize(chat, requestSettings, boundary);
  }

  async function generateImage(prompt: string, attachments: Attachment[] = []) {
    if (generating) return;
    const requestSettings = snapshotSettings();
    if (!requestSettings.active_proxy_id || !requestSettings.active_model) {
      lastError = $tr("chat.pickProxyAndModel");
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
      const title = prompt || attachments[0]?.name || $tr("chat.image");
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
      lastError = $tr("chat.forkIncognitoError");
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
      lastError = $tr("chat.pickProxyAndModel");
      return;
    }
    if (!msg.image_url && isImageGenerationModel(requestSettings)) {
      lastError = $tr("chat.imageModelInChatError");
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
    const messages = buildMessages(contextSlice, requestSettings, $activeChat.summary ?? null);
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
    let streamLimitReached = false;
    try {
      if (requestStreaming) {
        addVariation(msg.id, "", requestSettings.active_model);
        streamVariationAdded = true;
        const streamId = uid();
        activeStreamId = streamId;
        await api.streamCompletion(input, streamId, (ev) => {
          if (ev.type === "chunk") {
            if (streamLimitReached) return;
            const next = acc + ev.content;
            const limited = limitAssistantMessage(next, requestSettings);
            acc = limited;
            updateMessageContent(msg.id, limited);
            if (limited !== next) {
              streamLimitReached = true;
              void api.cancelGeneration(streamId);
            }
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
          updateMessageContent(msg.id, limitAssistantMessage(resp.content, requestSettings));
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

  async function onSaveSummary(content: string) {
    updateSummaryContent(content);
    await persistActive();
  }

  async function onDeleteSummary() {
    deleteSummary();
    await persistActive();
  }
</script>

{#if $incognito}
  <div class="incognito-banner">
    <Ghost size={14} fill="currentColor" />
    {$tr("chat.incognitoBanner")}
  </div>
{/if}
{#if $imageMode}
  <div class="imgmode-banner">
    <ImagePlusIcon size={14} />
    {$tr("chat.imageModeBanner")}
  </div>
{/if}

<div class="chat-area">
  {#if !$activeChat || $activeChat.messages.length === 0}
    <div class="empty-state">
      <div class="empty-card">
        <div class="empty-logo">Scarlet</div>
        <div class="empty-sub">{$tr("chat.emptySubtitle")}</div>
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
        {#if $activeChat.summary?.after_message_id === msg.id}
          <SummaryBoundary
            summary={$activeChat.summary}
            onSave={onSaveSummary}
            onDelete={onDeleteSummary}
          />
        {/if}
      {/each}
      {#if generating && !generationStreaming && !isRegenerating && !isImageGenerating}
        <div class="msg-group">
          <div class="message">
            <div class="role">{$settings.assistant_name || "Scarlet"}</div>
            {#if isSummarizing}
              <div class="typing-label">{$tr("chat.summarizing")}</div>
            {:else}
              <div class="typing">
                <span></span><span></span><span></span>
              </div>
            {/if}
          </div>
        </div>
      {/if}
    </div>
  {/if}
</div>

{#if lastError}
  <div class="error" role="alert">
    <span class="error-text">{lastError}</span>
    <button class="error-close" onclick={() => (lastError = null)} title={$tr("common.close")}>×</button>
  </div>
{/if}

<InputArea
  onSend={send}
  onStop={stop}
  busy={generating}
  canStop={canStopCurrent}
  canResend={lastMsgIsUser}
  {contextUsed}
  {contextWindow}
  onSummarize={summarizeChat}
  {canSummarize}
/>

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
  .typing-label {
    color: var(--text-2);
    font-size: 13px;
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
