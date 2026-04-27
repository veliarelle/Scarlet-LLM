<script lang="ts">
  import { Menu, Ghost, ImagePlus } from "lucide-svelte";
  import { sidebarOpen, incognito, imageMode } from "$lib/stores/ui";
  import { activeChat } from "$lib/stores/chats";
  import ModelSelector from "./ModelSelector.svelte";
  import ProxyIndicator from "./ProxyIndicator.svelte";

  // Инкогнито можно включить только если нет активного сохранённого чата.
  const canToggleIncognito = $derived(
    !$activeChat || $activeChat.id.startsWith("incognito-")
  );

  function toggleIncognito() {
    if (!canToggleIncognito) return;
    incognito.update((v) => !v);
  }
</script>

<div class="topbar">
  <button
    class="icon-btn mob-menu"
    onclick={() => sidebarOpen.update((v) => !v)}
    aria-label="Меню"
  >
    <Menu size={18} />
  </button>

  <div class="mid">
    <ModelSelector />
    <ProxyIndicator />
  </div>

  <div class="right">
    <button
      class="icon-btn desktop-only"
      title="Показать/скрыть боковую панель"
      onclick={() => sidebarOpen.update((v) => !v)}
      aria-label="Toggle sidebar"
    >
      <Menu size={18} />
    </button>

    <button
      class="icon-btn"
      class:active={$imageMode}
      title={$imageMode ? "Переключить в режим чата" : "Переключить в режим генерации изображений"}
      onclick={() => imageMode.update((v) => !v)}
      aria-label="Режим генерации изображений"
    >
      <ImagePlus size={18} />
    </button>

    <button
      class="icon-btn ghost"
      class:active={$incognito}
      class:disabled={!canToggleIncognito}
      title={!canToggleIncognito
        ? "Чат уже сохранён — создайте новый, чтобы включить инкогнито"
        : $incognito
          ? "Инкогнито включено"
          : "Включить инкогнито"}
      onclick={toggleIncognito}
      aria-label="Инкогнито"
      aria-disabled={!canToggleIncognito}
    >
      <Ghost size={18} fill={$incognito && canToggleIncognito ? "currentColor" : "none"} />
      {#if !canToggleIncognito}
        <span class="strike" aria-hidden="true"></span>
      {/if}
    </button>
  </div>
</div>

<style>
  .topbar {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 10px 12px;
    padding-top: calc(10px + env(safe-area-inset-top));
    border-bottom: 1px solid var(--border);
    min-height: 52px;
    flex-shrink: 0;
    background: var(--topbar-bg, var(--bg));
    backdrop-filter: blur(var(--topbar-blur, 0px));
    -webkit-backdrop-filter: blur(var(--topbar-blur, 0px));
    position: relative;
    z-index: 2;
  }
  .mid {
    display: flex;
    align-items: center;
    gap: 6px;
    flex: 1;
  }
  .right {
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .icon-btn {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 34px;
    height: 34px;
    border-radius: 8px;
    color: var(--text-3);
    transition: background 0.12s, color 0.12s;
    flex-shrink: 0;
  }
  .icon-btn:hover {
    background: var(--bg-3);
    color: var(--text-2);
  }
  .icon-btn.active {
    color: var(--accent);
  }
  .icon-btn.disabled {
    opacity: 0.45;
    cursor: not-allowed;
    color: var(--text-3);
  }
  .icon-btn.disabled:hover {
    background: transparent;
    color: var(--text-3);
  }
  .strike {
    position: absolute;
    top: 50%;
    left: 6px;
    right: 6px;
    height: 2px;
    background: currentColor;
    transform: rotate(-30deg);
    transform-origin: center;
    pointer-events: none;
    border-radius: 1px;
  }

  .mob-menu {
    display: none;
  }
  @media (max-width: 767px) {
    .mob-menu {
      display: flex;
    }
  }
  .desktop-only {
    display: flex;
  }
  @media (max-width: 767px) {
    .desktop-only {
      display: none;
    }
  }
</style>
