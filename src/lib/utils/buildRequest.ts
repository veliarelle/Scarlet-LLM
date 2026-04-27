import type { ChatMessage, ContentPart, Message } from "$lib/types/chat";
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

export function buildMessages(
  chatMessages: Message[],
  settings: Settings
): ChatMessage[] {
  const promptMessages: ChatMessage[] = settings.prompts
    .filter((p) => p.enabled && p.content.trim() !== "")
    .map((p) => ({
      role: p.role,
      content: applyTemplate(p.content, settings),
    }));
  const chat: ChatMessage[] = chatMessages.map((m) => {
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
  });
  return [...promptMessages, ...chat];
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
