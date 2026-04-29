<script lang="ts">
  import { tick } from "svelte";
  import { Bookmark as BookmarkIcon, Ghost, ImagePlus as ImagePlusIcon } from "lucide-svelte";
  import { api } from "$lib/api/invoke";
  import { tr } from "$lib/i18n";
  import { settings } from "$lib/stores/settings";
  import { activeGenerationId, bookmarksPanelOpen, imageMode, incognito } from "$lib/stores/ui";
  import {
    activeChat,
    appendToMessage,
    branchInfo,
    createSiblingBranch,
    createUserBranchFromEdit,
    deleteMessage,
    deleteMessageGroup,
    deleteSummary,
    ensureChat,
    forkActiveAt,
    getVisibleMessages,
    jumpToBookmark,
    persistActive,
    pushMessage,
    removeMessageById,
    rewindToMessage,
    selectSibling,
    setSummary,
    setMessageImageUrl,
    toggleBookmark,
    updateSummaryContent,
    updateMessageContent,
  } from "$lib/stores/chats";
  import type { Attachment, Chat, ChatMessage, CompletionResponse, Message, ToolCall } from "$lib/types/chat";
  import { uid } from "$lib/utils/id";
  import { buildMessages, buildParams, buildTools, estimateContextTokens, transcriptFromMessages } from "$lib/utils/buildRequest";
  import { executeToolCalls, providerToolsForAgent } from "$lib/utils/tools";
  import { applyAgentPrompt, applyAgentSettings, resolveAgentForRequest } from "$lib/utils/agents";
  import { DEFAULT_SUMMARIZE_PROMPT, type AgentDefinition, type Settings } from "$lib/types/settings";
  import MessageBubble from "./MessageBubble.svelte";
  import InputArea from "./InputArea.svelte";
  import SummaryBoundary from "./SummaryBoundary.svelte";

  let generating = $state(false);
  let generationStreaming = $state(false);
  let isRegenerating = $state(false);
  let isImageGenerating = $state(false);
  let isSummarizing = $state(false);
  let isAgentRunning = $state(false);
  let lastError = $state<string | null>(null);
  let activeStreamId = $state<string | null>(null);
  let chatAreaEl: HTMLDivElement | undefined = $state();
  let highlightedMessageId = $state<string | null>(null);
  let highlightTimer: ReturnType<typeof setTimeout> | null = null;
  let summarizeStopRequested = false;
  let agentStopRequested = false;
  const MAX_AGENT_STEPS = 6;

  const canStopCurrent = $derived(generating && (activeStreamId !== null || isAgentRunning));
  const visibleMessages = $derived(getVisibleMessages($activeChat));
  const lastMsgIsUser = $derived(visibleMessages[visibleMessages.length - 1]?.role === "user");
  const contextUsed = $derived(estimateContextTokens(visibleMessages, $settings, $activeChat?.summary ?? null));
  const contextWindow = $derived(Math.max(0, Number($settings.context_window ?? 0)));
  const canSummarize = $derived(!!$activeChat && visibleMessages.length > 0 && !generating);

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
      agent_definitions: (s.agent_definitions ?? []).map((agent) => ({
        ...agent,
        tool_names: [...(agent.tool_names ?? [])],
      })),
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

  function completionInput(messages: ChatMessage[], requestSettings: Settings, tools = buildTools(requestSettings)) {
    return {
      proxy_id: requestSettings.active_proxy_id!,
      model: requestSettings.active_model!,
      messages,
      params: buildParams(requestSettings),
      tools,
      web_search: requestSettings.web_search,
      prompt_caching: requestSettings.prompt_caching,
    };
  }

  function toolCalls(resp: CompletionResponse): ToolCall[] {
    return resp.tool_calls ?? [];
  }

  function toolCallFallback(resp: CompletionResponse, requestSettings: Settings): string {
    const calls = toolCalls(resp);
    if (calls.length === 0) return resp.content;
    if (requestSettings.agents) return resp.content;
    const names = calls.map((c) => c.name).join(", ");
    return resp.content || $tr("chat.toolCallsNeedAgents", { names });
  }

  async function runAgentCompletion(
    initialMessages: ChatMessage[],
    requestSettings: Settings,
    tools: unknown[],
    agent: AgentDefinition | null = null
  ): Promise<CompletionResponse> {
    const working: ChatMessage[] = [...initialMessages];
    let last: CompletionResponse | null = null;

    for (let step = 0; step < MAX_AGENT_STEPS; step += 1) {
      if (agentStopRequested) throw new Error("generation cancelled");

      const resp = await api.sendCompletionCancellable(
        completionInput(working, requestSettings, tools),
        activeStreamId ?? uid()
      );
      last = resp;
      const calls = toolCalls(resp);
      if (calls.length === 0) return resp;

      working.push({
        role: "assistant",
        content: resp.content ?? "",
        tool_calls: calls,
      });

      const results = await executeToolCalls(calls, requestSettings, agent);
      for (const result of results) {
        working.push({
          role: "tool",
          content: result.content,
          name: result.call.name,
          tool_call_id: result.call.id,
        });
      }
    }

    return {
      content: last?.content || $tr("chat.agentStepLimit"),
      usage: last?.usage ?? null,
      image_url: last?.image_url ?? null,
      tool_calls: last?.tool_calls ?? [],
    };
  }

  async function send(text: string, attachments: Attachment[] = []) {
    if (generating) return;
    if ($imageMode) {
      if (text.trim() || attachments.length) await generateImage(text.trim(), attachments);
      return;
    }
    const baseSettings = snapshotSettings();
    const selectedAgent = resolveAgentForRequest(text, baseSettings);
    const requestSettings = applyAgentSettings(baseSettings, selectedAgent);
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

    const messages = applyAgentPrompt(
      buildMessages(getVisibleMessages($activeChat), requestSettings, $activeChat?.summary ?? null),
      selectedAgent
    );
    const tools = selectedAgent ? providerToolsForAgent(requestSettings, selectedAgent) : buildTools(requestSettings);
    const useAgentLoop = requestSettings.agents && tools.length > 0;
    const input = completionInput(messages, requestSettings, tools);

    generating = true;
    generationStreaming = requestStreaming && tools.length === 0;
    isAgentRunning = useAgentLoop;
    agentStopRequested = false;
    let streamPlaceholderId: string | null = null;
    try {
      if (useAgentLoop) {
        activeStreamId = uid();
        const resp = await runAgentCompletion(messages, requestSettings, tools, selectedAgent);
        const msg = pushMessage(
          "assistant",
          limitAssistantMessage(toolCallFallback(resp, requestSettings), requestSettings),
          requestSettings.active_model
        );
        if (resp.image_url) {
          setMessageImageUrl(msg.id, resp.image_url);
        }
        await persistActive();
      } else if (requestStreaming && tools.length === 0) {
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
        const requestId = uid();
        activeStreamId = requestId;
        const resp = await api.sendCompletionCancellable(input, requestId);
        const msg = pushMessage(
          "assistant",
          limitAssistantMessage(toolCallFallback(resp, requestSettings), requestSettings),
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
      isAgentRunning = false;
      agentStopRequested = false;
      activeStreamId = null;
      activeGenerationId.set(null);
    }
  }

  async function stop() {
    if (isAgentRunning) {
      agentStopRequested = true;
    }
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
    const visible = getVisibleMessages(chat);
    const boundaryIdx = boundaryId
      ? visible.findIndex((m) => m.id === boundaryId)
      : visible.length - 1;
    const endIdx = boundaryIdx === -1 ? visible.length - 1 : boundaryIdx;
    const targetMessages = visible.slice(0, endIdx + 1);
    const existing = chat.summary;
    if (existing?.content.trim()) {
      const idx = visible.findIndex((m) => m.id === existing.after_message_id);
      if (idx !== -1 && idx <= endIdx) {
        return [
          "Existing summary of earlier conversation:",
          existing.content.trim(),
          "",
          "Conversation after that summary:",
          transcriptFromMessages(visible.slice(idx + 1, endIdx + 1)),
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
          prompt_caching: requestSettings.prompt_caching,
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
    return limit > 0 && estimateContextTokens(getVisibleMessages(chat), requestSettings, chat.summary ?? null) > limit;
  }

  function autoSummarizeBoundary(chat: Chat): Message | null {
    const visible = getVisibleMessages(chat);
    if (visible.length <= 1) return null;
    const last = visible[visible.length - 1];
    const boundaryIndex = last.role === "user" ? visible.length - 2 : visible.length - 1;
    if (boundaryIndex < 0) return null;
    const boundary = visible[boundaryIndex];
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
    if (generating || !$activeChat || visibleMessages.length === 0) return;
    const chat = $activeChat;
    const requestSettings = snapshotSettings();
    if (!requestSettings.active_proxy_id || !requestSettings.active_model) {
      lastError = $tr("chat.pickProxyAndModel");
      return;
    }

    const path = getVisibleMessages(chat);
    const boundary = path[path.length - 1];
    if (!boundary) return;
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

  async function onDeleteMessageGroup(id: string) {
    deleteMessageGroup(id);
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

  async function onPrevBranch(msg: Message) {
    selectSibling(msg.id, -1);
    await persistActive();
  }

  async function onNextBranch(msg: Message) {
    const info = branchInfo($activeChat, msg.id);
    if (msg.role === "assistant" && info.index >= info.count - 1) {
      await regenerateMessage(msg);
      return;
    }
    selectSibling(msg.id, 1);
    await persistActive();
  }

  async function onToggleBookmark(id: string) {
    toggleBookmark(id);
    await persistActive();
  }

  async function onJumpBookmark(messageId: string, leafId?: string | null) {
    jumpToBookmark(messageId, leafId);
    await tick();
    const target = Array.from(chatAreaEl?.querySelectorAll<HTMLElement>("[data-message-id]") ?? [])
      .find((el) => el.dataset.messageId === messageId);
    target?.scrollIntoView({ block: "center", behavior: "smooth" });
    highlightedMessageId = messageId;
    if (highlightTimer) clearTimeout(highlightTimer);
    highlightTimer = setTimeout(() => {
      highlightedMessageId = null;
      highlightTimer = null;
    }, 1000);
    await persistActive();
  }

  function onBookmarkWheel(e: WheelEvent) {
    const el = e.currentTarget as HTMLElement | null;
    if (!el || el.scrollWidth <= el.clientWidth) return;
    e.preventDefault();
    el.scrollLeft += Math.abs(e.deltaX) > Math.abs(e.deltaY) ? e.deltaX : e.deltaY;
  }

  async function onSendEditedUser(id: string, content: string) {
    const created = createUserBranchFromEdit(id, content);
    if (!created) return;
    await persistActive();
    await send("", []);
  }

  async function regenerateMessage(msg: Message) {
    if (generating) return;
    const baseSettings = snapshotSettings();
    const selectedAgent = resolveAgentForRequest(msg.content, baseSettings);
    const requestSettings = applyAgentSettings(baseSettings, selectedAgent);
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
      const branch = createSiblingBranch(msg.id, "", requestSettings.active_model, null);
      if (!branch) {
        generating = false;
        generationStreaming = false;
        isImageGenerating = false;
        activeStreamId = null;
        activeGenerationId.set(null);
        return;
      }
      try {
        const result = await api.generateImage({
          proxy_id: requestSettings.active_proxy_id,
          model: requestSettings.active_model,
          prompt: msg.content,
          image_id: imageId,
          params: buildParams(requestSettings),
        });
        setMessageImageUrl(branch.id, result.url);
        updateMessageContent(branch.id, msg.content);
        await persistActive();
      } catch (e) {
        if (!isCancelled(e)) {
          lastError = e instanceof Error ? e.message : String(e);
        }
        deleteMessage(branch.id);
        rewindToMessage(msg.id);
      } finally {
        generating = false;
        generationStreaming = false;
        isImageGenerating = false;
        activeStreamId = null;
        activeGenerationId.set(null);
      }
      return;
    }

    const path = getVisibleMessages($activeChat);
    const idx = path.findIndex((m) => m.id === msg.id);
    if (idx === -1) return;
    const contextSlice = path.slice(0, idx);
    const messages = applyAgentPrompt(
      buildMessages(contextSlice, requestSettings, $activeChat.summary ?? null),
      selectedAgent
    );
    const tools = selectedAgent ? providerToolsForAgent(requestSettings, selectedAgent) : buildTools(requestSettings);
    const useAgentLoop = requestSettings.agents && tools.length > 0;
    const input = completionInput(messages, requestSettings, tools);

    generating = true;
    generationStreaming = requestStreaming && tools.length === 0;
    isRegenerating = true;
    isAgentRunning = useAgentLoop;
    agentStopRequested = false;
    let acc = "";
    let branchId: string | null = null;
    let streamLimitReached = false;
    try {
      if (useAgentLoop) {
        activeStreamId = uid();
        const branch = createSiblingBranch(msg.id, "", requestSettings.active_model);
        branchId = branch?.id ?? null;
        if (!branchId) throw new Error("failed to create branch");
        try {
          const resp = await runAgentCompletion(messages, requestSettings, tools, selectedAgent);
          updateMessageContent(branchId, limitAssistantMessage(toolCallFallback(resp, requestSettings), requestSettings));
          if (resp.image_url) {
            setMessageImageUrl(branchId, resp.image_url);
          }
          await persistActive();
        } catch (inner) {
          deleteMessage(branchId);
          rewindToMessage(msg.id);
          throw inner;
        }
      } else if (requestStreaming && tools.length === 0) {
        const branch = createSiblingBranch(msg.id, "", requestSettings.active_model);
        branchId = branch?.id ?? null;
        if (!branchId) throw new Error("failed to create branch");
        const streamId = uid();
        activeStreamId = streamId;
        await api.streamCompletion(input, streamId, (ev) => {
          if (ev.type === "chunk") {
            if (streamLimitReached) return;
            const next = acc + ev.content;
            const limited = limitAssistantMessage(next, requestSettings);
            acc = limited;
            if (branchId) updateMessageContent(branchId, limited);
            if (limited !== next) {
              streamLimitReached = true;
              void api.cancelGeneration(streamId);
            }
          } else if (ev.type === "error") {
            lastError = ev.message;
          }
        });
        if (acc === "") {
          deleteMessage(branchId);
          rewindToMessage(msg.id);
          branchId = null;
        } else {
          await persistActive();
        }
      } else {
        const branch = createSiblingBranch(msg.id, "", requestSettings.active_model);
        branchId = branch?.id ?? null;
        if (!branchId) throw new Error("failed to create branch");
        try {
          const requestId = uid();
          activeStreamId = requestId;
          const resp = await api.sendCompletionCancellable(input, requestId);
          updateMessageContent(branchId, limitAssistantMessage(toolCallFallback(resp, requestSettings), requestSettings));
          if (resp.image_url) {
            setMessageImageUrl(branchId, resp.image_url);
          }
          await persistActive();
        } catch (inner) {
          deleteMessage(branchId);
          rewindToMessage(msg.id);
          throw inner;
        }
      }
    } catch (e) {
      if (!isCancelled(e)) {
        lastError = e instanceof Error ? e.message : String(e);
      }
      if (branchId) {
        const branch = $activeChat?.messages.find((m) => m.id === branchId);
        if (branch && branch.content === "" && !branch.image_url) {
          deleteMessage(branchId);
          rewindToMessage(msg.id);
        }
      }
    } finally {
      generating = false;
      generationStreaming = false;
      isRegenerating = false;
      isAgentRunning = false;
      agentStopRequested = false;
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

{#if $bookmarksPanelOpen && $activeChat && ($activeChat.bookmarks ?? []).length > 0}
  <div class="bookmark-strip" aria-label={$tr("chat.bookmarks")} onwheel={onBookmarkWheel}>
    <span class="bookmark-icon" title={$tr("chat.bookmarks")}>
      <BookmarkIcon size={14} fill="currentColor" />
    </span>
    {#each $activeChat.bookmarks ?? [] as bookmark (bookmark.id)}
      <button class="bookmark-chip" onclick={() => onJumpBookmark(bookmark.message_id, bookmark.leaf_id)} title={bookmark.label}>
        {bookmark.label}
      </button>
    {/each}
  </div>
{/if}

<div
  class="chat-area"
  class:with-bookmarks={$bookmarksPanelOpen && $activeChat && ($activeChat.bookmarks ?? []).length > 0}
  bind:this={chatAreaEl}
>
  {#if !$activeChat || visibleMessages.length === 0}
    <div class="empty-state">
      <div class="empty-card">
        <div class="empty-logo">Scarlet</div>
        <div class="empty-sub">{$tr("chat.emptySubtitle")}</div>
      </div>
    </div>
  {:else}
    <div class="chat-inner">
      {#each visibleMessages as msg (msg.id)}
        <div class="message-anchor" data-message-id={msg.id}>
          <MessageBubble
            {msg}
            branchIndex={branchInfo($activeChat, msg.id).index}
            branchCount={branchInfo($activeChat, msg.id).count}
            branchLocked={generating}
            highlighted={highlightedMessageId === msg.id}
            onEdit={(c) => onEditMessage(msg.id, c)}
            onSendEdit={(c) => onSendEditedUser(msg.id, c)}
            onDelete={() => onDeleteMessage(msg.id)}
            onDeleteGroup={() => onDeleteMessageGroup(msg.id)}
            onRewind={() => onRewindMessage(msg.id)}
            onFork={() => onForkMessage(msg.id)}
            onPrevBranch={() => onPrevBranch(msg)}
            onNextBranch={() => onNextBranch(msg)}
            onRegenerate={() => regenerateMessage(msg)}
            onToggleBookmark={() => onToggleBookmark(msg.id)}
          />
        </div>
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
    min-height: 0;
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
  .chat-area.with-bookmarks {
    padding-top: 28px;
    -webkit-mask-image: linear-gradient(
      to bottom,
      transparent 0,
      black 48px,
      black calc(100% - 28px),
      transparent 100%
    );
    mask-image: linear-gradient(
      to bottom,
      transparent 0,
      black 48px,
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
  .message-anchor {
    width: 100%;
  }
  .bookmark-strip {
    position: relative;
    z-index: 3;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: 6px;
    flex-wrap: nowrap;
    margin: 10px 24px 0;
    padding: 8px 10px;
    border: 1px solid var(--border);
    border-radius: 10px;
    background: color-mix(in srgb, var(--bg) 88%, var(--bg-3));
    box-shadow: 0 12px 18px color-mix(in srgb, var(--bg) 78%, transparent);
    overflow-x: auto;
    overflow-y: hidden;
    overscroll-behavior-inline: contain;
    scrollbar-width: thin;
    -webkit-overflow-scrolling: touch;
  }
  .bookmark-icon {
    flex-shrink: 0;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    border-radius: 6px;
    color: var(--text-3);
    background: var(--bg-4);
  }
  .bookmark-chip {
    flex: 0 0 auto;
    max-width: min(260px, 58vw);
    min-height: 34px;
    padding: 6px 10px;
    border-radius: 7px;
    background: var(--bg-4);
    color: var(--text-2);
    font-size: 12px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .bookmark-chip:hover {
    color: var(--text);
    background: color-mix(in srgb, var(--accent) 15%, var(--bg-4));
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
    flex-shrink: 0;
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
