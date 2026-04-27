<script lang="ts">
  import { X } from "lucide-svelte";
  import { tr } from "$lib/i18n";
  import { settingsOpen } from "$lib/stores/ui";
  import GeneralTab from "./GeneralTab.svelte";
  import ParametersTab from "./ParametersTab.svelte";
  import ReasoningTab from "./ReasoningTab.svelte";
  import PromptsTab from "./PromptsTab.svelte";
  import ToolsTab from "./ToolsTab.svelte";
  import AppearanceTab from "./AppearanceTab.svelte";

  type Tab = "general" | "params" | "reasoning" | "prompts" | "tools" | "appearance";
  let tab = $state<Tab>("general");

  const TABS = $derived<{ id: Tab; label: string }[]>([
    { id: "general", label: $tr("settings.tabs.general") },
    { id: "params", label: $tr("settings.tabs.params") },
    { id: "reasoning", label: $tr("settings.tabs.reasoning") },
    { id: "prompts", label: $tr("settings.tabs.prompts") },
    { id: "tools", label: $tr("settings.tabs.tools") },
    { id: "appearance", label: $tr("settings.tabs.appearance") },
  ]);

  function close() {
    settingsOpen.set(false);
  }
  function onOverlay(e: MouseEvent) {
    if (e.target === e.currentTarget) close();
  }
</script>

{#if $settingsOpen}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="overlay"
    onclick={onOverlay}
    onkeydown={(e) => e.key === "Escape" && close()}
  >
    <div class="drawer" role="dialog" aria-modal="true" aria-label={$tr("settings.title")}>
      <div class="header">
        <span>{$tr("settings.title")}</span>
        <button class="close-btn" onclick={close} aria-label={$tr("common.close")}>
          <X size={18} />
        </button>
      </div>

      <div class="tabs">
        {#each TABS as t (t.id)}
          <button
            class="tab"
            class:active={tab === t.id}
            onclick={() => (tab = t.id)}
          >
            {t.label}
          </button>
        {/each}
      </div>

      <div class="body">
        {#if tab === "general"}
          <GeneralTab />
        {:else if tab === "params"}
          <ParametersTab />
        {:else if tab === "reasoning"}
          <ReasoningTab />
        {:else if tab === "prompts"}
          <PromptsTab />
        {:else if tab === "tools"}
          <ToolsTab />
        {:else if tab === "appearance"}
          <AppearanceTab />
        {/if}
      </div>
    </div>
  </div>
{/if}

<style>
  .overlay {
    position: fixed;
    inset: 0;
    z-index: 300;
    background: rgba(0, 0, 0, 0.55);
    backdrop-filter: blur(2px);
    display: flex;
    justify-content: flex-end;
    animation: fadeIn 0.15s;
  }
  .drawer {
    width: min(480px, 100%);
    background: var(--bg-2);
    border-left: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    animation: slideIn 0.22s cubic-bezier(0.4, 0, 0.2, 1);
  }
  @keyframes slideIn {
    from {
      transform: translateX(100%);
    }
    to {
      transform: translateX(0);
    }
  }
  @keyframes fadeIn {
    from {
      opacity: 0;
    }
    to {
      opacity: 1;
    }
  }

  .header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px 18px;
    border-bottom: 1px solid var(--border);
    font-size: 15px;
    font-weight: 600;
  }
  .close-btn {
    color: var(--text-3);
    display: flex;
  }
  .close-btn:hover {
    color: var(--text);
  }

  .tabs {
    display: flex;
    gap: 2px;
    padding: 8px 10px;
    border-bottom: 1px solid var(--border);
    overflow-x: auto;
    scrollbar-width: none;
  }
  .tab {
    padding: 5px 11px;
    border-radius: 7px;
    font-size: 12px;
    color: var(--text-3);
    white-space: nowrap;
    transition: background 0.1s, color 0.1s;
  }
  .tab:hover {
    color: var(--text-2);
    background: var(--bg-3);
  }
  .tab.active {
    background: var(--bg-4);
    color: var(--text);
  }

  .body {
    flex: 1;
    overflow-y: auto;
    padding: 14px 18px;
    display: flex;
    flex-direction: column;
    gap: 18px;
  }
</style>
