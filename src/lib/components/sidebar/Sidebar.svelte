<script lang="ts">
  import { Search, X, Plus, Pin, Pencil, Trash2, MoreVertical } from "lucide-svelte";
  import { onMount } from "svelte";
  import { tr } from "$lib/i18n";
  import { activeGenerationId, sidebarOpen } from "$lib/stores/ui";
  import { api } from "$lib/api/invoke";
  import {
    activeChat,
    chatList,
    deleteChatById,
    pinChatById,
    refreshList,
    renameChatById,
    selectChat,
  } from "$lib/stores/chats";
  import type { ChatMeta } from "$lib/types/chat";
  import { clickOutside } from "$lib/utils/clickOutside";

  let searchOpen = $state(false);
  let search = $state("");

  let contextMenu = $state<{ chatId: string; x: number; y: number } | null>(null);
  let renamingId = $state<string | null>(null);
  let renameValue = $state("");
  let longPressTimer: ReturnType<typeof setTimeout> | null = null;
  let overlayPointerDown = false;

  onMount(async () => {
    await refreshList();
  });

  function close() {
    sidebarOpen.set(false);
  }

  function onOverlayPointerDown(e: PointerEvent) {
    overlayPointerDown = e.target === e.currentTarget;
  }

  function onOverlayClick(e: MouseEvent) {
    const shouldClose = overlayPointerDown && e.target === e.currentTarget;
    overlayPointerDown = false;
    if (shouldClose) close();
  }

  async function stopActiveGeneration() {
    if ($activeGenerationId) {
      await api.cancelGeneration($activeGenerationId);
      activeGenerationId.set(null);
    }
  }

  async function newChat() {
    await stopActiveGeneration();
    await selectChat(null);
    if (window.innerWidth < 768) close();
  }

  async function openChat(id: string) {
    if ($activeChat?.id === id) {
      if (window.innerWidth < 768) close();
      return;
    }
    await stopActiveGeneration();
    await selectChat(id);
    if (window.innerWidth < 768) close();
  }

  function showMenu(e: MouseEvent, chatId: string) {
    e.preventDefault();
    e.stopPropagation();
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
    contextMenu = { chatId, x: r.right, y: r.top };
  }

  function onLongPressStart(e: TouchEvent, id: string) {
    const t = e.touches[0];
    longPressTimer = setTimeout(() => {
      contextMenu = { chatId: id, x: t.clientX, y: t.clientY };
    }, 500);
  }

  function onLongPressEnd() {
    if (longPressTimer) {
      clearTimeout(longPressTimer);
      longPressTimer = null;
    }
  }

  async function togglePin() {
    if (!contextMenu) return;
    const c = $chatList.find((x) => x.id === contextMenu!.chatId);
    if (!c) return;
    await pinChatById(c.id, !c.pinned);
    contextMenu = null;
  }

  function startRename() {
    if (!contextMenu) return;
    const c = $chatList.find((x) => x.id === contextMenu!.chatId);
    if (!c) return;
    renamingId = c.id;
    renameValue = c.title;
    contextMenu = null;
  }

  async function commitRename(id: string) {
    const v = renameValue.trim();
    if (v) await renameChatById(id, v);
    renamingId = null;
  }

  async function deleteChat() {
    if (!contextMenu) return;
    const id = contextMenu.chatId;
    contextMenu = null;
    if (!confirm($tr("sidebar.deleteConfirm"))) return;
    await deleteChatById(id);
  }

  const filtered = $derived(
    $chatList.filter((c) => {
      if (!search.trim()) return true;
      return c.title.toLowerCase().includes(search.toLowerCase());
    })
  );
  const sorted = $derived(
    [...filtered].sort((a, b) => {
      if (a.pinned !== b.pinned) return a.pinned ? -1 : 1;
      return b.updated_at.localeCompare(a.updated_at);
    })
  );
  const pinned = $derived(sorted.filter((c) => c.pinned));
  const unpinned = $derived(sorted.filter((c) => !c.pinned));
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="overlay"
  class:visible={$sidebarOpen}
  onpointerdown={onOverlayPointerDown}
  onclick={onOverlayClick}
  aria-hidden="true"
></div>

<aside class="sidebar" class:open={$sidebarOpen} class:collapsed={!$sidebarOpen}>
  <div class="header">
    <span class="logo">Scarlet</span>
    <div class="header-actions">
      <button
        class="icon-btn"
        class:active={searchOpen}
        title={$tr("common.search")}
        onclick={() => {
          searchOpen = !searchOpen;
          if (!searchOpen) search = "";
        }}
      >
        <Search size={16} />
      </button>
      <button class="icon-btn close" title={$tr("common.close")} onclick={close}>
        <X size={18} />
      </button>
    </div>
  </div>

  {#if searchOpen}
    <div class="search-wrap">
      <Search size={14} color="var(--text-3)" />
      <input class="search-input" bind:value={search} placeholder={$tr("sidebar.searchPlaceholder")} />
      {#if search}
        <button onclick={() => (search = "")} aria-label={$tr("sidebar.clear")}>
          <X size={14} color="var(--text-3)" />
        </button>
      {/if}
    </div>
  {/if}

  <button class="new-chat-btn" onclick={newChat}>
    <Plus size={16} />
    {$tr("sidebar.newChat")}
  </button>

  <div class="chat-list">
    {#if pinned.length > 0}
      <div class="group-label">{$tr("sidebar.pinned")}</div>
      {#each pinned as c (c.id)}
        {@render chatItem(c)}
      {/each}
      <div class="group-label">{$tr("sidebar.chats")}</div>
    {/if}
    {#each unpinned as c (c.id)}
      {@render chatItem(c)}
    {/each}
    {#if sorted.length === 0}
      <div class="chat-empty">{search ? $tr("sidebar.noResults") : $tr("sidebar.empty")}</div>
    {/if}
  </div>
</aside>

{#snippet chatItem(c: ChatMeta)}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="chat-item"
    class:active={$activeChat?.id === c.id}
    onclick={() => openChat(c.id)}
    oncontextmenu={(e) => showMenu(e, c.id)}
    ontouchstart={(e) => onLongPressStart(e, c.id)}
    ontouchend={onLongPressEnd}
    ontouchmove={onLongPressEnd}
  >
    {#if c.pinned}
      <Pin size={11} color="var(--accent)" fill="currentColor" />
    {/if}
    {#if renamingId === c.id}
      <!-- svelte-ignore a11y_autofocus -->
      <input
        class="rename-input"
        bind:value={renameValue}
        onkeydown={(e) => {
          if (e.key === "Enter") commitRename(c.id);
          if (e.key === "Escape") (renamingId = null);
        }}
        onblur={() => commitRename(c.id)}
        onclick={(e) => e.stopPropagation()}
        autofocus
      />
    {:else}
      <span class="chat-title">{c.title}</span>
    {/if}
    <button
      class="menu-btn"
      onclick={(e) => showMenu(e, c.id)}
      aria-label={$tr("common.menu")}
    >
      <MoreVertical size={14} color="var(--text-3)" />
    </button>
  </div>
{/snippet}

{#if contextMenu}
  <div
    class="context-menu"
    use:clickOutside={() => (contextMenu = null)}
    style="left: {Math.min(contextMenu.x, window.innerWidth - 160)}px; top: {Math.min(
      contextMenu.y,
      window.innerHeight - 140
    )}px"
  >
    <button onclick={togglePin}>
      <Pin size={14} />
      {$chatList.find((c) => c.id === contextMenu!.chatId)?.pinned ? $tr("sidebar.unpin") : $tr("sidebar.pin")}
    </button>
    <button onclick={startRename}>
      <Pencil size={14} /> {$tr("sidebar.rename")}
    </button>
    <button class="danger" onclick={deleteChat}>
      <Trash2 size={14} /> {$tr("common.delete")}
    </button>
  </div>
{/if}

<style>
  .sidebar {
    width: 256px;
    flex-shrink: 0;
    background: var(--sidebar-bg, var(--bg-2));
    backdrop-filter: blur(var(--sidebar-blur, 0px));
    -webkit-backdrop-filter: blur(var(--sidebar-blur, 0px));
    border-right: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    transition: width 0.25s cubic-bezier(0.4, 0, 0.2, 1), opacity 0.2s, border-color 0.25s;
    overflow: hidden;
  }
  .sidebar.collapsed {
    width: 0;
    border-color: transparent;
    opacity: 0;
  }
  @media (max-width: 767px) {
    .sidebar {
      position: fixed;
      left: 0;
      top: 0;
      bottom: 0;
      z-index: 50;
      width: 256px;
      opacity: 1;
      transform: translateX(-100%);
      transition: transform 0.25s cubic-bezier(0.4, 0, 0.2, 1);
    }
    .sidebar.open {
      transform: translateX(0);
    }
    .sidebar.collapsed {
      transform: translateX(-100%);
      width: 256px;
      opacity: 1;
      border-color: var(--border);
    }
  }

  .overlay {
    display: none;
    position: fixed;
    inset: 0;
    z-index: 40;
    background: rgba(0, 0, 0, 0.5);
    opacity: 0;
    transition: opacity 0.25s;
    pointer-events: none;
  }
  @media (max-width: 767px) {
    .overlay {
      display: block;
    }
    .overlay.visible {
      opacity: 1;
      pointer-events: all;
    }
  }

  .header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px 14px 10px;
    flex-shrink: 0;
  }
  .logo {
    font-size: 15px;
    font-weight: 700;
    letter-spacing: 0.02em;
    color: var(--accent);
  }
  .header-actions {
    display: flex;
    gap: 4px;
  }
  .icon-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 30px;
    height: 30px;
    border-radius: 8px;
    color: var(--text-3);
    transition: background 0.12s, color 0.12s;
  }
  .icon-btn:hover {
    background: var(--bg-3);
    color: var(--text-2);
  }
  .icon-btn.active {
    color: var(--accent);
  }
  .icon-btn.close {
    display: none;
  }
  @media (max-width: 767px) {
    .icon-btn.close {
      display: flex;
    }
  }

  .search-wrap {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 0 10px 6px;
    padding: 7px 10px;
    background: var(--bg-3);
    border: 1px solid var(--border);
    border-radius: 8px;
  }
  .search-input {
    flex: 1;
    font-size: 13px;
  }
  .search-input::placeholder {
    color: var(--text-3);
  }

  .new-chat-btn {
    margin: 0 10px 6px;
    padding: 8px 12px;
    border-radius: 8px;
    background: var(--bg-3);
    border: 1px solid var(--border);
    color: var(--text-2);
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
    flex-shrink: 0;
    transition: background 0.12s, color 0.12s;
  }
  .new-chat-btn:hover {
    background: var(--bg-4);
    color: var(--text);
  }

  .chat-list {
    flex: 1;
    overflow-y: auto;
    padding: 2px 8px 12px;
  }
  .group-label {
    font-size: 10px;
    color: var(--text-3);
    text-transform: uppercase;
    letter-spacing: 0.08em;
    padding: 8px 8px 4px;
  }
  .chat-empty {
    color: var(--text-3);
    font-size: 13px;
    padding: 16px 8px;
    text-align: center;
  }

  .chat-item {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 7px 10px;
    border-radius: 8px;
    cursor: pointer;
    min-height: 34px;
    transition: background 0.1s;
    user-select: none;
    /* content-visibility — браузер не рендерит элементы вне viewport;
       помогает при списках в 10к+ чатов без виртуализации. */
    content-visibility: auto;
    contain-intrinsic-size: 0 36px;
  }
  .chat-item:hover {
    background: var(--bg-3);
  }
  .chat-item.active {
    background: var(--bg-4);
  }
  .chat-title {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 13px;
    color: var(--text-2);
  }
  .chat-item.active .chat-title {
    color: var(--text);
  }
  .menu-btn {
    opacity: 0;
    padding: 2px;
    display: flex;
  }
  .chat-item:hover .menu-btn,
  .chat-item.active .menu-btn {
    opacity: 1;
  }
  .rename-input {
    flex: 1;
    background: var(--bg-4);
    border-radius: 4px;
    padding: 1px 6px;
    font-size: 13px;
    border: 1px solid var(--border);
    color: var(--text);
  }

  .context-menu {
    position: fixed;
    z-index: 200;
    background: var(--bg-3);
    border: 1px solid var(--border);
    border-radius: 10px;
    padding: 4px;
    min-width: 150px;
    box-shadow: var(--shadow);
  }
  .context-menu button {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 8px 12px;
    border-radius: 6px;
    color: var(--text-2);
    font-size: 13px;
    transition: background 0.1s;
  }
  .context-menu button:hover {
    background: var(--bg-4);
    color: var(--text);
  }
  .context-menu button.danger:hover {
    color: var(--danger);
  }
</style>
