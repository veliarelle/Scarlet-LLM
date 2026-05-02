<script lang="ts">
  import { onMount } from "svelte";
  import { proxies } from "$lib/stores/proxies";
  import { settings } from "$lib/stores/settings";
  import { models } from "$lib/stores/models";
  import { refreshList, selectChat } from "$lib/stores/chats";
  import { COLOR_VARS } from "$lib/types/settings";
  import { applyBackground } from "$lib/utils/background";
  import Sidebar from "$lib/components/sidebar/Sidebar.svelte";
  import TopBar from "$lib/components/topbar/TopBar.svelte";
  import ChatView from "$lib/components/chat/ChatView.svelte";
  import SettingsDrawer from "$lib/components/settings/SettingsDrawer.svelte";

  function uiScale(): number {
    const raw = Number($settings.ui_scale ?? 1);
    if (!Number.isFinite(raw)) return 1;
    return Math.min(1.5, Math.max(0.75, raw));
  }
  function textScale(): number {
    const raw = Number($settings.text_scale ?? 1);
    if (!Number.isFinite(raw)) return 1;
    return Math.min(1.5, Math.max(0.75, raw));
  }

  const scaleValue = $derived(String(uiScale()));

  function syncVisualViewport() {
    if (typeof document === "undefined" || typeof window === "undefined") return;
    const root = document.documentElement;
    const vv = window.visualViewport;
    root.style.setProperty("--visual-vw", `${vv?.width ?? window.innerWidth}px`);
    root.style.setProperty("--visual-vh", `${vv?.height ?? window.innerHeight}px`);
    root.style.setProperty("--visual-top", `${vv?.offsetTop ?? 0}px`);
    root.style.setProperty("--visual-left", `${vv?.offsetLeft ?? 0}px`);
  }

  // Применяем тему + кастомные цвета + ui_scale + translucency
  $effect(() => {
    if (typeof document === "undefined") return;
    const root = document.documentElement;
    const baseTheme = $settings.theme === "custom" ? "dark" : $settings.theme;
    root.setAttribute("data-theme", baseTheme);

    for (const v of COLOR_VARS) {
      root.style.removeProperty(`--${v}`);
    }
    if ($settings.theme === "custom") {
      for (const [k, v] of Object.entries($settings.custom_colors)) {
        if (v) root.style.setProperty(`--${k}`, v);
      }
    }

    root.style.setProperty("--ui-scale", String($settings.ui_scale ?? 1));
    root.style.setProperty("--text-scale", String(textScale()));

    // Translucency сайдбара/топбара — выставляем CSS-переменные,
    // которые потребляют компоненты Sidebar/TopBar.
    if ($settings.translucent_sidebar) {
      root.style.setProperty(
        "--sidebar-bg",
        "color-mix(in srgb, var(--bg-2) 55%, transparent)"
      );
      root.style.setProperty("--sidebar-blur", `${$settings.sidebar_blur}px`);
    } else {
      root.style.setProperty("--sidebar-bg", "var(--bg-2)");
      root.style.setProperty("--sidebar-blur", "0px");
    }
    if ($settings.translucent_topbar) {
      root.style.setProperty(
        "--topbar-bg",
        "color-mix(in srgb, var(--bg) 55%, transparent)"
      );
      root.style.setProperty("--topbar-blur", `${$settings.topbar_blur}px`);
    } else {
      root.style.setProperty("--topbar-bg", "var(--bg)");
      root.style.setProperty("--topbar-blur", "0px");
    }
  });

  onMount(() => {
    syncVisualViewport();
    window.addEventListener("resize", syncVisualViewport);
    window.visualViewport?.addEventListener("resize", syncVisualViewport);
    window.visualViewport?.addEventListener("scroll", syncVisualViewport);
    void (async () => {
      // Применяем фон ДО загрузки настроек, чтобы изображение было видно
      // сразу после старта приложения (загружается из localStorage).
      applyBackground();

      await Promise.all([proxies.load(), settings.load(), refreshList()]);
      if ($settings.active_proxy_id) {
        models.load($settings.active_proxy_id);
      }
      if ($settings.active_chat_id) {
        try {
          await selectChat($settings.active_chat_id);
        } catch {
          // чат удалён руками — игнорируем
        }
      }
    })();
    return () => {
      window.removeEventListener("resize", syncVisualViewport);
      window.visualViewport?.removeEventListener("resize", syncVisualViewport);
      window.visualViewport?.removeEventListener("scroll", syncVisualViewport);
    };
  });
</script>

<div class="viewport">
  <div
    class="app-scale"
    style={`--app-scale: ${scaleValue};`}
  >
    <div class="root">
      <Sidebar />
      <main class="main">
        <TopBar />
        <ChatView />
      </main>
    </div>
  </div>

  <div
    class="overlay-scale"
    style={`--app-scale: ${scaleValue};`}
  >
    <SettingsDrawer />
  </div>
</div>

<style>
  .viewport {
    position: fixed;
    top: var(--visual-top, 0px);
    left: var(--visual-left, 0px);
    z-index: 1;
    width: var(--visual-vw, 100vw);
    height: var(--visual-vh, 100dvh);
    overflow: hidden;
  }
  .app-scale {
    position: absolute;
    inset: 0;
    width: calc(var(--visual-vw, 100vw) / var(--app-scale));
    height: calc(var(--visual-vh, 100dvh) / var(--app-scale));
    overflow: hidden;
    transform: scale(var(--app-scale));
    transform-origin: top left;
  }
  .overlay-scale {
    position: absolute;
    inset: 0;
    z-index: 500;
    width: calc(var(--visual-vw, 100vw) / var(--app-scale));
    height: calc(var(--visual-vh, 100dvh) / var(--app-scale));
    pointer-events: none;
    transform: scale(var(--app-scale));
    transform-origin: top left;
  }
  .overlay-scale :global(.overlay) {
    pointer-events: auto;
  }
  .root {
    display: flex;
    width: 100%;
    height: 100%;
    overflow: hidden;
    position: relative;
  }
  .main {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
  }
</style>
