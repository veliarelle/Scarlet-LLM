<script lang="ts">
  import { onMount } from "svelte";
  import { proxies } from "$lib/stores/proxies";
  import { settings } from "$lib/stores/settings";
  import { models } from "$lib/stores/models";
  import { refreshList, selectChat } from "$lib/stores/chats";
  import { applyBackground } from "$lib/utils/background";
  import Sidebar from "$lib/components/sidebar/Sidebar.svelte";
  import TopBar from "$lib/components/topbar/TopBar.svelte";
  import ChatView from "$lib/components/chat/ChatView.svelte";
  import SettingsDrawer from "$lib/components/settings/SettingsDrawer.svelte";

  let viewportWidth = $state(0);
  let viewportHeight = $state(0);

  function uiScale(): number {
    const raw = Number($settings.ui_scale ?? 1);
    return Number.isFinite(raw) && raw > 0 ? raw : 1;
  }

  function updateViewport() {
    if (typeof window === "undefined") return;
    viewportWidth = window.innerWidth;
    viewportHeight = window.innerHeight;
  }

  const scaledWidth = $derived(viewportWidth > 0 ? `${viewportWidth / uiScale()}px` : "100vw");
  const scaledHeight = $derived(viewportHeight > 0 ? `${viewportHeight / uiScale()}px` : "100vh");
  const scaleValue = $derived(String(uiScale()));
  const scaleTransform = $derived(`scale(${scaleValue})`);

  // Применяем тему + кастомные цвета + ui_scale + translucency
  $effect(() => {
    if (typeof document === "undefined") return;
    const root = document.documentElement;
    const baseTheme = $settings.theme === "custom" ? "dark" : $settings.theme;
    root.setAttribute("data-theme", baseTheme);

    const ALL_VARS = [
      "bg",
      "bg-2",
      "bg-3",
      "bg-4",
      "border",
      "accent",
      "accent-h",
      "accent-d",
      "text",
      "text-2",
      "text-3",
      "danger",
    ];
    for (const v of ALL_VARS) {
      root.style.removeProperty(`--${v}`);
    }
    if ($settings.theme === "custom") {
      for (const [k, v] of Object.entries($settings.custom_colors)) {
        if (v) root.style.setProperty(`--${k}`, v);
      }
    }

    root.style.setProperty("--ui-scale", String($settings.ui_scale ?? 1));

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
    updateViewport();
    window.addEventListener("resize", updateViewport);

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
      window.removeEventListener("resize", updateViewport);
    };
  });
</script>

<div class="viewport">
  <div
    class="app-scale"
    style:width={scaledWidth}
    style:height={scaledHeight}
    style:transform={scaleTransform}
  >
    <div class="root">
      <Sidebar />
      <main class="main">
        <TopBar />
        <ChatView />
      </main>
    </div>

    <SettingsDrawer />
  </div>
</div>

<style>
  .viewport {
    position: fixed;
    inset: 0;
    z-index: 1;
    width: 100vw;
    height: 100vh;
    height: 100dvh;
    overflow: hidden;
  }
  .app-scale {
    position: relative;
    overflow: hidden;
    transform-origin: top left;
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
