import type { ChatMessage, ChatSummary, ContentPart, Message } from "$lib/types/chat";
import type { Settings } from "$lib/types/settings";

function parseValue(v: string): unknown {
  if (v === "true") return true;
  if (v === "false") return false;
  if (v === "null") return null;
  const trimmed = v.trim();
  if (trimmed !== "" && !isNaN(Number(trimmed))) return Number(trimmed);
  // try JSON parse for arrays/objects
  if (trimmed.startsWith("{") || trimmed.startsWith("[") || trimmed.startsWith('"')) {
    try {
      return JSON.parse(trimmed);
    } catch {
      // fall through
    }
  }
  return v;
}

function applyTemplate(content: string, settings: Settings): string {
  return content
    .replace(/\{\{user\}\}/g, settings.user_name)
    .replace(/\{\{assistant\}\}/g, settings.assistant_name);
}

export const CHARS_PER_TOKEN = 4;
export const IMAGE_TOKEN_ESTIMATE = 1024;

export function estimateTextTokens(text: string): number {
  return Math.ceil(text.length / CHARS_PER_TOKEN);
}

export function estimatePartTokens(part: ContentPart): number {
  if (part.type === "text") return estimateTextTokens(part.text);
  if (part.type === "image_url") return IMAGE_TOKEN_ESTIMATE;
  return Math.ceil(part.source.data.length / CHARS_PER_TOKEN);
}

export function estimateChatMessageTokens(message: ChatMessage): number {
  const contentTokens = typeof message.content === "string"
    ? estimateTextTokens(message.content)
    : message.content.reduce((sum, part) => sum + estimatePartTokens(part), 0);
  return contentTokens + 4;
}

function truncateTextToTokens(text: string, tokenBudget: number): string {
  const maxChars = Math.max(0, tokenBudget * CHARS_PER_TOKEN);
  if (text.length <= maxChars) return text;
  if (maxChars <= 16) return text.slice(-maxChars);
  return `[truncated]\n${text.slice(-(maxChars - 12))}`;
}

function truncateMessageToTokens(message: ChatMessage, tokenBudget: number): ChatMessage {
  const budget = Math.max(1, tokenBudget - 4);
  if (typeof message.content === "string") {
    return { ...message, content: truncateTextToTokens(message.content, budget) };
  }

  const parts: ContentPart[] = [];
  let used = 0;
  for (let i = message.content.length - 1; i >= 0; i -= 1) {
    const part = message.content[i];
    const cost = estimatePartTokens(part);
    if (used + cost <= budget) {
      parts.unshift(part);
      used += cost;
      continue;
    }
    if (part.type === "text" && used < budget) {
      parts.unshift({ ...part, text: truncateTextToTokens(part.text, budget - used) });
    }
    break;
  }
  return { ...message, content: parts.length > 0 ? parts : message.content.slice(-1) };
}

function applyContextWindow(messages: ChatMessage[], settings: Settings): ChatMessage[] {
  const budget = Math.floor(Number(settings.context_window ?? 0));
  if (!Number.isFinite(budget) || budget <= 0) return messages;

  const selected: ChatMessage[] = [];
  let used = 0;
  for (let i = messages.length - 1; i >= 0; i -= 1) {
    const message = messages[i];
    const cost = estimateChatMessageTokens(message);
    if (used + cost <= budget) {
      selected.unshift(message);
      used += cost;
      continue;
    }
    if (selected.length === 0) {
      selected.unshift(truncateMessageToTokens(message, budget - used));
    }
    break;
  }
  return selected;
}

function applySummaryBoundary(messages: Message[], summary?: ChatSummary | null): {
  messages: Message[];
  summaryMessage: ChatMessage | null;
} {
  if (!summary?.content.trim()) return { messages, summaryMessage: null };
  const idx = messages.findIndex((m) => m.id === summary.after_message_id);
  if (idx === -1) return { messages, summaryMessage: null };
  return {
    messages: messages.slice(idx + 1),
    summaryMessage: {
      role: "system",
      content: `Summary of earlier conversation:\n${summary.content.trim()}`,
    },
  };
}

function messageToChatMessage(m: Message): ChatMessage {
  const atts = m.attachments ?? [];
  if (atts.length === 0) return { role: m.role, content: m.content };
  const parts: ContentPart[] = [];
  if (m.content) parts.push({ type: "text", text: m.content });
  for (const att of atts) {
    if (att.mimeType.startsWith("image/")) {
      parts.push({ type: "image_url", image_url: { url: `data:${att.mimeType};base64,${att.data}` } });
    } else {
      parts.push({ type: "file", source: { type: "base64", media_type: att.mimeType, data: att.data }, name: att.name });
    }
  }
  return { role: m.role, content: parts };
}

export function estimateStoredMessageTokens(message: Message): number {
  return estimateChatMessageTokens(messageToChatMessage(message));
}

function buildPromptMessages(settings: Settings): ChatMessage[] {
  return settings.prompts
    .filter((p) => p.enabled && p.content.trim() !== "")
    .map((p) => ({
      role: p.role,
      content: applyTemplate(p.content, settings),
    }));
}

export function buildFullMessages(
  chatMessages: Message[],
  settings: Settings,
  summary?: ChatSummary | null
): ChatMessage[] {
  const promptMessages = buildPromptMessages(settings);
  const boundary = applySummaryBoundary(chatMessages, summary);
  const chat: ChatMessage[] = boundary.messages.map(messageToChatMessage);
  return [
    ...promptMessages,
    ...(boundary.summaryMessage ? [boundary.summaryMessage] : []),
    ...chat,
  ];
}

export function estimateContextTokens(
  chatMessages: Message[],
  settings: Settings,
  summary?: ChatSummary | null
): number {
  return buildFullMessages(chatMessages, settings, summary).reduce(
    (sum, message) => sum + estimateChatMessageTokens(message),
    0
  );
}

export function buildMessages(
  chatMessages: Message[],
  settings: Settings,
  summary?: ChatSummary | null
): ChatMessage[] {
  const budget = Math.floor(Number(settings.context_window ?? 0));
  if (!Number.isFinite(budget) || budget <= 0) {
    return buildFullMessages(chatMessages, settings, summary);
  }

  const promptMessages = buildPromptMessages(settings);
  const boundary = applySummaryBoundary(chatMessages, summary);
  const fixedMessages = [
    ...promptMessages,
    ...(boundary.summaryMessage ? [boundary.summaryMessage] : []),
  ];
  const fixedCost = fixedMessages.reduce((sum, message) => sum + estimateChatMessageTokens(message), 0);
  const chatBudget = budget - fixedCost;
  const chat = boundary.messages.map(messageToChatMessage);

  if (chatBudget <= 0) {
    return applyContextWindow(fixedMessages, settings);
  }

  return [
    ...fixedMessages,
    ...applyContextWindow(chat, { ...settings, context_window: chatBudget }),
  ];
}

export function transcriptFromMessages(messages: Message[]): string {
  return messages
    .map((m) => {
      const role = m.role.toUpperCase();
      const attachments = (m.attachments ?? [])
        .map((a) => `[attachment: ${a.name}, ${a.mimeType}]`)
        .join("\n");
      const image = m.image_url ? "[generated image]" : "";
      return [role, m.content, attachments, image].filter(Boolean).join("\n");
    })
    .join("\n\n---\n\n");
}

export function buildParams(settings: Settings): Record<string, unknown> {
  const out: Record<string, unknown> = {};
  if (settings.max_tokens > 0) {
    out.max_tokens = settings.max_tokens;
  }
  for (const p of settings.params) {
    if (!p.enabled || !p.key.trim()) continue;
    out[p.key] = parseValue(p.value);
  }
  if (settings.reasoning.enabled && settings.reasoning.send_effort) {
    out.reasoning_effort = settings.reasoning.effort;
  }
  return out;
}

export function buildTools(settings: Settings): unknown[] {
  if (!settings.tools) return [];
  try {
    const defs = JSON.parse(settings.tool_definitions || "[]");
    return Array.isArray(defs) ? defs : [];
  } catch {
    return [];
  }
}
