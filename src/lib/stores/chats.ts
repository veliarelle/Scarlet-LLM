import { get, writable } from "svelte/store";
import { api } from "$lib/api/invoke";
import type { Attachment, Chat, ChatMeta, Message, Role, VariationMeta } from "$lib/types/chat";
import { uid } from "$lib/utils/id";
import { settings } from "./settings";
import { incognito } from "./ui";

export const chatList = writable<ChatMeta[]>([]);
export const activeChat = writable<Chat | null>(null);

function nowIso(): string {
  return new Date().toISOString();
}

function isIncognito(): boolean {
  return get(incognito);
}

async function persist(chat: Chat) {
  if (isIncognito()) return;
  await api.saveChat(chat);
  await refreshList();
}

export async function refreshList() {
  const list = await api.listChats();
  chatList.set(list);
}

function applyMetaToList(meta: ChatMeta) {
  chatList.update((list) => {
    const idx = list.findIndex((c) => c.id === meta.id);
    if (idx === -1) return [...list, meta];
    const next = [...list];
    next[idx] = meta;
    return next;
  });
}

export async function selectChat(id: string | null) {
  if (id === null) {
    activeChat.set(null);
    await settings.patch({ active_chat_id: null });
    return;
  }
  try {
    const c = await api.loadChat(id);
    activeChat.set(c);
    // Открытие сохранённого чата автоматически выключает инкогнито
    incognito.set(false);
    await settings.patch({ active_chat_id: id });
  } catch (e) {
    activeChat.set(null);
    await settings.patch({ active_chat_id: null });
    throw e;
  }
}

export async function ensureChat(initialUserContent: string): Promise<Chat> {
  const curr = get(activeChat);
  if (curr) return curr;

  const title = initialUserContent.slice(0, 40) + (initialUserContent.length > 40 ? "…" : "");
  const s = get(settings);

  if (isIncognito()) {
    const c: Chat = {
      id: `incognito-${uid()}`,
      title,
      pinned: false,
      created_at: nowIso(),
      updated_at: nowIso(),
      model: s.active_model ?? null,
      proxy_id: s.active_proxy_id ?? null,
      messages: [],
    };
    activeChat.set(c);
    return c;
  }

  const created = await api.createChat({
    title,
    model: s.active_model ?? null,
    proxy_id: s.active_proxy_id ?? null,
  });
  activeChat.set(created);
  await settings.patch({ active_chat_id: created.id });
  await refreshList();
  return created;
}

export function pushMessage(role: Role, content: string, model?: string | null, attachments?: Attachment[]): Message {
  const now = nowIso();
  const isAsst = role === "assistant";
  const meta: VariationMeta = { model: model ?? null, created_at: now };
  const msg: Message = {
    id: uid(),
    role,
    content,
    created_at: now,
    model: model ?? null,
    variations: isAsst ? [content] : [],
    variation_index: 0,
    variation_meta: isAsst ? [meta] : [],
    attachments: attachments && attachments.length > 0 ? attachments : undefined,
  };
  activeChat.update((c) => {
    if (!c) return c;
    return { ...c, messages: [...c.messages, msg], updated_at: nowIso() };
  });
  return msg;
}

export function appendToMessage(messageId: string, delta: string) {
  activeChat.update((c) => {
    if (!c) return c;
    const next = { ...c, messages: [...c.messages] };
    const idx = next.messages.findIndex((m) => m.id === messageId);
    if (idx === -1) return c;
    const m = next.messages[idx];
    const newContent = m.content + delta;
    // Также обновляем активную вариацию, если она существует и совпадает с текущим content
    const variations = m.variations ? [...m.variations] : [];
    const vIdx = m.variation_index ?? 0;
    if (variations.length > vIdx) {
      variations[vIdx] = newContent;
    }
    next.messages[idx] = { ...m, content: newContent, variations };
    return next;
  });
}

export function selectVariation(messageId: string, index: number) {
  activeChat.update((c) => {
    if (!c) return c;
    const next = { ...c, messages: [...c.messages] };
    const idx = next.messages.findIndex((m) => m.id === messageId);
    if (idx === -1) return c;
    const m = next.messages[idx];
    const variations = m.variations ?? [];
    if (index < 0 || index >= variations.length) return c;
    const meta = (m.variation_meta ?? [])[index];
    next.messages[idx] = {
      ...m,
      content: variations[index],
      variation_index: index,
      image_url: meta?.image_url ?? null,
    };
    return next;
  });
}

export function addVariation(
  messageId: string,
  content: string,
  model?: string | null,
  imageUrl?: string | null,
) {
  const meta: VariationMeta = { model: model ?? null, created_at: nowIso(), image_url: imageUrl ?? null };
  activeChat.update((c) => {
    if (!c) return c;
    const next = { ...c, messages: [...c.messages] };
    const idx = next.messages.findIndex((m) => m.id === messageId);
    if (idx === -1) return c;
    const m = next.messages[idx];
    const variations = [...(m.variations ?? []), content];
    const variation_meta = [...(m.variation_meta ?? []), meta];
    next.messages[idx] = {
      ...m,
      content,
      image_url: imageUrl ?? null,
      variations,
      variation_index: variations.length - 1,
      variation_meta,
    };
    return next;
  });
}

export function popVariation(messageId: string) {
  activeChat.update((c) => {
    if (!c) return c;
    const next = { ...c, messages: [...c.messages] };
    const idx = next.messages.findIndex((m) => m.id === messageId);
    if (idx === -1) return c;
    const m = next.messages[idx];
    const variations = m.variations ?? [];
    if (variations.length <= 1) return c;
    const newVariations = variations.slice(0, -1);
    const newMeta = (m.variation_meta ?? []).slice(0, -1);
    const newIdx = newVariations.length - 1;
    const prevMeta = newMeta[newIdx];
    next.messages[idx] = {
      ...m,
      content: newVariations[newIdx] ?? "",
      image_url: prevMeta?.image_url ?? null,
      variations: newVariations,
      variation_index: newIdx,
      variation_meta: newMeta,
    };
    return next;
  });
}

export function updateMessageContent(messageId: string, content: string) {
  activeChat.update((c) => {
    if (!c) return c;
    const next = {
      ...c,
      messages: c.messages.map((m) => {
        if (m.id !== messageId) return m;
        // Если есть вариации — обновляем активную тоже, чтобы они не разъезжались.
        const variations = m.variations ? [...m.variations] : [];
        const vIdx = m.variation_index ?? 0;
        if (variations.length > vIdx) {
          variations[vIdx] = content;
        }
        return { ...m, content, variations };
      }),
    };
    return next;
  });
}

export function setMessageImageUrl(messageId: string, imageUrl: string) {
  activeChat.update((c) => {
    if (!c) return c;
    return {
      ...c,
      messages: c.messages.map((m) => {
        if (m.id !== messageId) return m;
        const variation_meta = m.variation_meta ? [...m.variation_meta] : [];
        const vIdx = m.variation_index ?? 0;
        if (variation_meta.length > vIdx) {
          variation_meta[vIdx] = { ...variation_meta[vIdx], image_url: imageUrl };
        }
        return { ...m, image_url: imageUrl, variation_meta };
      }),
    };
  });
}

export function deleteMessage(messageId: string) {
  activeChat.update((c) => {
    if (!c) return c;
    return { ...c, messages: c.messages.filter((m) => m.id !== messageId), updated_at: nowIso() };
  });
}

export function rewindToMessage(messageId: string) {
  activeChat.update((c) => {
    if (!c) return c;
    const idx = c.messages.findIndex((m) => m.id === messageId);
    if (idx === -1) return c;
    return { ...c, messages: c.messages.slice(0, idx + 1), updated_at: nowIso() };
  });
}

export function removeMessageById(messageId: string) {
  activeChat.update((c) => {
    if (!c) return c;
    return { ...c, messages: c.messages.filter((m) => m.id !== messageId) };
  });
}

export async function persistActive() {
  const c = get(activeChat);
  if (!c) return;
  await persist(c);
}

export async function renameActive(title: string) {
  activeChat.update((c) => (c ? { ...c, title } : c));
  if (!isIncognito()) {
    const c = get(activeChat);
    if (c && !c.id.startsWith("incognito-")) {
      const meta = await api.renameChat(c.id, title);
      applyMetaToList(meta);
    }
  }
}

export async function pinChatById(id: string, pinned: boolean) {
  if (id.startsWith("incognito-")) return;
  const meta = await api.pinChat(id, pinned);
  applyMetaToList(meta);
  activeChat.update((c) => (c && c.id === id ? { ...c, pinned } : c));
}

export async function renameChatById(id: string, title: string) {
  if (id.startsWith("incognito-")) return;
  const meta = await api.renameChat(id, title);
  applyMetaToList(meta);
  activeChat.update((c) => (c && c.id === id ? { ...c, title } : c));
}

export async function deleteChatById(id: string) {
  if (!id.startsWith("incognito-")) {
    await api.deleteChat(id);
  }
  chatList.update((list) => list.filter((c) => c.id !== id));
  const curr = get(activeChat);
  if (curr && curr.id === id) {
    activeChat.set(null);
    await settings.patch({ active_chat_id: null });
  }
}

export async function forkActiveAt(messageId: string) {
  const c = get(activeChat);
  if (!c) return;
  if (c.id.startsWith("incognito-")) return; // нельзя форкнуть инкогнито на диск
  const newChat = await api.forkChat(c.id, messageId);
  activeChat.set(newChat);
  await settings.patch({ active_chat_id: newChat.id });
  await refreshList();
}
