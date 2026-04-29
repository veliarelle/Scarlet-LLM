import { get, writable } from "svelte/store";
import { api } from "$lib/api/invoke";
import { tr } from "$lib/i18n";
import type { Attachment, Chat, ChatBookmark, ChatMeta, ChatSummary, Message, Role, VariationMeta } from "$lib/types/chat";
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

function cloneMessage(m: Message): Message {
  return {
    ...m,
    parent_id: m.parent_id ?? null,
    child_ids: [...(m.child_ids ?? [])],
    active_child_id: m.active_child_id ?? null,
    variations: m.variations ? [...m.variations] : undefined,
    variation_meta: m.variation_meta ? m.variation_meta.map((meta) => ({ ...meta })) : undefined,
    attachments: m.attachments ? m.attachments.map((att) => ({ ...att })) : undefined,
  };
}

function messageMap(messages: Message[]): Map<string, Message> {
  return new Map(messages.map((m) => [m.id, m]));
}

function roots(messages: Message[]): Message[] {
  return messages.filter((m) => !m.parent_id);
}

function branchLeafFrom(chat: Chat, startId: string): string {
  const map = messageMap(chat.messages);
  let curr = map.get(startId);
  const seen = new Set<string>();
  while (curr && !seen.has(curr.id)) {
    seen.add(curr.id);
    const active = curr.active_child_id && map.get(curr.active_child_id);
    const fallback = (curr.child_ids ?? []).map((id) => map.get(id)).find(Boolean);
    const next = active ?? fallback;
    if (!next) return curr.id;
    curr = next;
  }
  return startId;
}

function pathToMessage(chat: Chat, messageId: string | null | undefined): Message[] {
  if (!messageId) return [];
  const map = messageMap(chat.messages);
  const path: Message[] = [];
  const seen = new Set<string>();
  let curr = map.get(messageId);
  while (curr && !seen.has(curr.id)) {
    seen.add(curr.id);
    path.push(curr);
    curr = curr.parent_id ? map.get(curr.parent_id) : undefined;
  }
  return path.reverse();
}

export function getVisibleMessages(chat: Chat | null): Message[] {
  if (!chat || chat.messages.length === 0) return [];
  const leaf = chat.active_leaf_id && chat.messages.some((m) => m.id === chat.active_leaf_id)
    ? chat.active_leaf_id
    : branchLeafFrom(chat, roots(chat.messages)[0]?.id ?? chat.messages[0].id);
  const path = pathToMessage(chat, leaf);
  return path.length > 0 ? path : chat.messages.slice(0, 1);
}

function activeLeaf(chat: Chat): Message | null {
  const visible = getVisibleMessages(chat);
  return visible[visible.length - 1] ?? null;
}

function setActivePath(chat: Chat, messageId: string, includeDescendants: boolean): Chat {
  const targetLeaf = includeDescendants ? branchLeafFrom(chat, messageId) : messageId;
  const path = pathToMessage(chat, targetLeaf);
  return {
    ...chat,
    active_leaf_id: targetLeaf,
    messages: chat.messages.map((m) => {
      const idx = path.findIndex((item) => item.id === m.id);
      if (idx === -1 || idx >= path.length - 1) return m;
      return { ...m, active_child_id: path[idx + 1].id };
    }),
    updated_at: nowIso(),
  };
}

function legacyNodesForMessage(message: Message, parentId: string | null): { nodes: Message[]; activeId: string } {
  const variations = message.role === "assistant" ? message.variations ?? [] : [];
  if (variations.length <= 1) {
    return {
      nodes: [
        {
          ...message,
          parent_id: parentId,
          child_ids: [],
          active_child_id: null,
          variations: message.role === "assistant" ? [message.content] : message.variations,
          variation_index: 0,
          variation_meta: message.variation_meta && message.variation_meta.length > 0
            ? [message.variation_meta[message.variation_index ?? 0] ?? message.variation_meta[0]]
            : message.variation_meta,
        },
      ],
      activeId: message.id,
    };
  }

  const activeIndex = Math.min(Math.max(0, message.variation_index ?? 0), variations.length - 1);
  const nodes = variations.map((content, idx) => {
    const meta = message.variation_meta?.[idx];
    const id = idx === activeIndex ? message.id : `${message.id}-branch-${idx}`;
    return {
      ...message,
      id,
      content,
      parent_id: parentId,
      child_ids: [],
      active_child_id: null,
      model: meta?.model ?? message.model ?? null,
      image_url: meta?.image_url ?? (idx === activeIndex ? message.image_url ?? null : null),
      variations: [content],
      variation_index: 0,
      variation_meta: meta ? [{ ...meta }] : [],
      bookmarked: idx === activeIndex ? message.bookmarked : false,
    };
  });
  return { nodes, activeId: nodes[activeIndex].id };
}

export function normalizeChatTree(chat: Chat): Chat {
  const messages = chat.messages.map(cloneMessage);
  const hasTree = messages.some(
    (m) => m.parent_id !== null || (m.child_ids ?? []).length > 0 || !!m.active_child_id
  ) || !!chat.active_leaf_id;

  if (!hasTree) {
    const linear: Message[] = [];
    let parentId: string | null = null;
    let previousActiveId: string | null = null;
    for (const message of messages) {
      const { nodes, activeId } = legacyNodesForMessage(message, parentId);
      if (previousActiveId) {
        const previous = linear.find((m) => m.id === previousActiveId);
        if (previous) {
          previous.child_ids = nodes.map((m) => m.id);
          previous.active_child_id = activeId;
        }
      }
      linear.push(...nodes);
      parentId = activeId;
      previousActiveId = activeId;
    }
    return {
      ...chat,
      messages: linear,
      active_leaf_id: chat.active_leaf_id ?? linear[linear.length - 1]?.id ?? null,
      bookmarks: chat.bookmarks ?? [],
    };
  }

  const map = messageMap(messages);
  const childIds = new Map<string, string[]>();
  for (const m of messages) {
    if (m.parent_id && map.has(m.parent_id)) {
      childIds.set(m.parent_id, [...(childIds.get(m.parent_id) ?? []), m.id]);
    } else {
      m.parent_id = null;
    }
  }
  for (const m of messages) {
    const fromParent = childIds.get(m.id) ?? [];
    const fromStored = (m.child_ids ?? []).filter((id) => map.has(id) && map.get(id)?.parent_id === m.id);
    const merged = Array.from(new Set([...fromStored, ...fromParent]));
    m.child_ids = merged;
    if (m.active_child_id && !merged.includes(m.active_child_id)) {
      m.active_child_id = merged[0] ?? null;
    }
  }

  const root = roots(messages)[0] ?? messages[0];
  const active = chat.active_leaf_id && map.has(chat.active_leaf_id)
    ? chat.active_leaf_id
    : root
      ? branchLeafFrom({ ...chat, messages }, root.id)
      : null;
  return { ...chat, messages, active_leaf_id: active, bookmarks: chat.bookmarks ?? [] };
}

async function persist(chat: Chat) {
  if (isIncognito()) return;
  await api.saveChat(normalizeChatTree(chat));
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
    const c = normalizeChatTree(await api.loadChat(id));
    activeChat.set(c);
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

  const titleSource = initialUserContent.trim() || get(tr)("sidebar.newChat");
  const title = titleSource.slice(0, 40) + (titleSource.length > 40 ? "…" : "");
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
      summary: null,
      active_leaf_id: null,
      bookmarks: [],
      messages: [],
    };
    activeChat.set(c);
    return c;
  }

  const created = normalizeChatTree(await api.createChat({
    title,
    model: s.active_model ?? null,
    proxy_id: s.active_proxy_id ?? null,
  }));
  activeChat.set(created);
  await settings.patch({ active_chat_id: created.id });
  await refreshList();
  return created;
}

function makeMessage(role: Role, content: string, model?: string | null, attachments?: Attachment[], parentId?: string | null): Message {
  const now = nowIso();
  const isAsst = role === "assistant";
  const meta: VariationMeta = { model: model ?? null, created_at: now };
  return {
    id: uid(),
    role,
    content,
    created_at: now,
    parent_id: parentId ?? null,
    child_ids: [],
    active_child_id: null,
    model: model ?? null,
    variations: isAsst ? [content] : [],
    variation_index: 0,
    variation_meta: isAsst ? [meta] : [],
    attachments: attachments && attachments.length > 0 ? attachments : undefined,
  };
}

function addMessageAsActiveChild(chat: Chat, msg: Message): Chat {
  const messages = [...chat.messages, msg].map((m) => {
    if (msg.parent_id && m.id === msg.parent_id) {
      const child_ids = [...(m.child_ids ?? []).filter((id) => id !== msg.id), msg.id];
      return { ...m, child_ids, active_child_id: msg.id };
    }
    return m;
  });
  return { ...chat, messages, active_leaf_id: msg.id, updated_at: nowIso() };
}

export function pushMessage(role: Role, content: string, model?: string | null, attachments?: Attachment[]): Message {
  const chat = get(activeChat);
  const parent = chat ? activeLeaf(chat) : null;
  const msg = makeMessage(role, content, model, attachments, parent?.id ?? null);
  activeChat.update((c) => (c ? addMessageAsActiveChild(c, msg) : c));
  return msg;
}

export function createSiblingBranch(messageId: string, content: string, model?: string | null, imageUrl?: string | null): Message | null {
  let created: Message | null = null;
  activeChat.update((c) => {
    if (!c) return c;
    const current = c.messages.find((m) => m.id === messageId);
    if (!current) return c;
    const msg = makeMessage(current.role, content, model ?? current.model ?? null, current.attachments, current.parent_id ?? null);
    msg.image_url = imageUrl ?? null;
    if (msg.variation_meta?.[0]) msg.variation_meta[0].image_url = imageUrl ?? null;
    created = msg;
    return addMessageAsActiveChild(c, msg);
  });
  return created;
}

export function createUserBranchFromEdit(messageId: string, content: string): Message | null {
  let created: Message | null = null;
  activeChat.update((c) => {
    if (!c) return c;
    const current = c.messages.find((m) => m.id === messageId && m.role === "user");
    if (!current) return c;
    const msg = makeMessage("user", content, null, current.attachments, current.parent_id ?? null);
    created = msg;
    return addMessageAsActiveChild(c, msg);
  });
  return created;
}

export function appendToMessage(messageId: string, delta: string) {
  activeChat.update((c) => {
    if (!c) return c;
    return {
      ...c,
      messages: c.messages.map((m) => {
        if (m.id !== messageId) return m;
        const content = m.content + delta;
        const variations = m.variations ? [...m.variations] : undefined;
        if (variations?.length) variations[0] = content;
        return { ...m, content, variations };
      }),
    };
  });
}

export function updateMessageContent(messageId: string, content: string) {
  activeChat.update((c) => {
    if (!c) return c;
    return {
      ...c,
      messages: c.messages.map((m) => {
        if (m.id !== messageId) return m;
        const variations = m.variations ? [...m.variations] : undefined;
        if (variations?.length) variations[0] = content;
        return { ...m, content, variations };
      }),
      updated_at: nowIso(),
    };
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
        if (variation_meta.length > 0) {
          variation_meta[0] = { ...variation_meta[0], image_url: imageUrl };
        }
        return { ...m, image_url: imageUrl, variation_meta };
      }),
      updated_at: nowIso(),
    };
  });
}

function collectSubtreeIds(messages: Message[], messageId: string): Set<string> {
  const map = messageMap(messages);
  const out = new Set<string>();
  const visit = (id: string) => {
    if (out.has(id)) return;
    out.add(id);
    for (const child of map.get(id)?.child_ids ?? []) visit(child);
  };
  visit(messageId);
  return out;
}

function removeMessageSubtrees(chat: Chat, targetIds: string[], currentId: string): Chat {
  const current = chat.messages.find((m) => m.id === currentId);
  if (!current) return chat;
  const currentSiblings = current.parent_id
    ? chat.messages.find((m) => m.id === current.parent_id)?.child_ids ?? []
    : roots(chat.messages).map((m) => m.id);
  const removed = new Set<string>();
  for (const id of targetIds) {
    for (const subtreeId of collectSubtreeIds(chat.messages, id)) {
      removed.add(subtreeId);
    }
  }
  const messages = chat.messages
    .filter((m) => !removed.has(m.id))
    .map((m) => ({
      ...m,
      child_ids: (m.child_ids ?? []).filter((id) => !removed.has(id)),
      active_child_id: m.active_child_id && !removed.has(m.active_child_id) ? m.active_child_id : null,
    }));
  const bookmarks = (chat.bookmarks ?? []).filter((b) => !removed.has(b.message_id));
  const summary = chat.summary && removed.has(chat.summary.after_message_id) ? null : chat.summary;
  const nextChat = { ...chat, messages };
  const currentIndex = currentSiblings.indexOf(currentId);
  const siblingExists = (id: string | undefined) => !!id && !removed.has(id) && messages.some((m) => m.id === id);
  let remainingSibling: string | undefined;
  for (let i = currentIndex - 1; i >= 0; i -= 1) {
    if (siblingExists(currentSiblings[i])) {
      remainingSibling = currentSiblings[i];
      break;
    }
  }
  if (!remainingSibling) {
    for (let i = Math.max(0, currentIndex + 1); i < currentSiblings.length; i += 1) {
      if (siblingExists(currentSiblings[i])) {
        remainingSibling = currentSiblings[i];
        break;
      }
    }
  }
  const nextLeaf = remainingSibling
    ? branchLeafFrom(nextChat, remainingSibling)
    : current.parent_id && messages.some((m) => m.id === current.parent_id)
      ? current.parent_id
      : roots(messages)[0]?.id ?? null;

  return { ...chat, messages, bookmarks, summary, active_leaf_id: nextLeaf, updated_at: nowIso() };
}

export function deleteMessage(messageId: string) {
  activeChat.update((c) => {
    if (!c) return c;
    return removeMessageSubtrees(c, [messageId], messageId);
  });
}

export function deleteMessageGroup(messageId: string) {
  activeChat.update((c) => {
    if (!c) return c;
    const current = c.messages.find((m) => m.id === messageId);
    if (!current) return c;
    const siblings = current.parent_id
      ? c.messages.find((m) => m.id === current.parent_id)?.child_ids ?? [messageId]
      : roots(c.messages).map((m) => m.id);
    return removeMessageSubtrees(c, siblings, messageId);
  });
}

export function removeMessageById(messageId: string) {
  deleteMessage(messageId);
}

export function selectMessagePath(messageId: string, includeDescendants = false) {
  activeChat.update((c) => (c ? setActivePath(c, messageId, includeDescendants) : c));
}

export function rewindToMessage(messageId: string) {
  selectMessagePath(messageId, false);
}

export function branchInfo(chat: Chat | null, messageId: string): { index: number; count: number } {
  if (!chat) return { index: 0, count: 1 };
  const msg = chat.messages.find((m) => m.id === messageId);
  if (!msg) return { index: 0, count: 1 };
  const siblings = msg.parent_id
    ? chat.messages.find((m) => m.id === msg.parent_id)?.child_ids ?? []
    : roots(chat.messages).map((m) => m.id);
  const index = Math.max(0, siblings.indexOf(messageId));
  return { index, count: Math.max(1, siblings.length) };
}

export function selectSibling(messageId: string, dir: -1 | 1) {
  activeChat.update((c) => {
    if (!c) return c;
    const msg = c.messages.find((m) => m.id === messageId);
    if (!msg) return c;
    const siblings = msg.parent_id
      ? c.messages.find((m) => m.id === msg.parent_id)?.child_ids ?? []
      : roots(c.messages).map((m) => m.id);
    const idx = siblings.indexOf(messageId);
    const nextId = siblings[idx + dir];
    if (!nextId) return c;
    return setActivePath(c, nextId, true);
  });
}

export function addVariation(messageId: string, content: string, model?: string | null, imageUrl?: string | null) {
  createSiblingBranch(messageId, content, model, imageUrl);
}

export function popVariation(messageId: string) {
  deleteMessage(messageId);
}

export async function persistActive() {
  const c = get(activeChat);
  if (!c) return;
  await persist(c);
}

export function setSummary(summary: ChatSummary) {
  activeChat.update((c) => (c ? { ...c, summary, updated_at: nowIso() } : c));
}

export function updateSummaryContent(content: string) {
  activeChat.update((c) => {
    if (!c?.summary) return c;
    return {
      ...c,
      summary: { ...c.summary, content, updated_at: nowIso() },
      updated_at: nowIso(),
    };
  });
}

export function deleteSummary() {
  activeChat.update((c) => (c ? { ...c, summary: null, updated_at: nowIso() } : c));
}

function bookmarkLabel(message: Message): string {
  const raw = message.content.trim() || (message.image_url ? "Image" : message.role);
  return raw.slice(0, 48) + (raw.length > 48 ? "..." : "");
}

export function toggleBookmark(messageId: string) {
  activeChat.update((c) => {
    if (!c) return c;
    const message = c.messages.find((m) => m.id === messageId);
    if (!message) return c;
    const existing = (c.bookmarks ?? []).find((b) => b.message_id === messageId);
    const currentLeaf = c.active_leaf_id && pathToMessage(c, c.active_leaf_id).some((m) => m.id === messageId)
      ? c.active_leaf_id
      : branchLeafFrom(c, messageId);
    const bookmarks: ChatBookmark[] = existing
      ? (c.bookmarks ?? []).filter((b) => b.message_id !== messageId)
      : [
          ...(c.bookmarks ?? []),
          {
            id: uid(),
            message_id: messageId,
            leaf_id: currentLeaf,
            label: bookmarkLabel(message),
            created_at: nowIso(),
          },
        ];
    return {
      ...c,
      bookmarks,
      messages: c.messages.map((m) => (m.id === messageId ? { ...m, bookmarked: !existing } : m)),
      updated_at: nowIso(),
    };
  });
}

export function jumpToBookmark(messageId: string, leafId?: string | null) {
  activeChat.update((c) => {
    if (!c) return c;
    const savedLeafPath = leafId ? pathToMessage(c, leafId) : [];
    if (savedLeafPath.some((m) => m.id === messageId)) {
      return setActivePath(c, leafId!, false);
    }
    if (c.messages.some((m) => m.id === messageId)) {
      return setActivePath(c, messageId, true);
    }
    return c;
  });
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
  if (c.id.startsWith("incognito-")) return;
  const newChat = await api.forkChat(c.id, messageId);
  activeChat.set(normalizeChatTree(newChat));
  await settings.patch({ active_chat_id: newChat.id });
  await refreshList();
}
