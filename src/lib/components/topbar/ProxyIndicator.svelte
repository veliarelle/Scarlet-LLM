<script lang="ts">
  import { Menu } from "lucide-svelte";
  import { tr } from "$lib/i18n";
  import { proxies } from "$lib/stores/proxies";
  import { settings } from "$lib/stores/settings";
  import { proxyPanelOpen } from "$lib/stores/ui";
  import ProxyPanel from "./ProxyPanel.svelte";

  const active = $derived(
    $proxies.find((p) => p.id === $settings.active_proxy_id)
  );
</script>

<div class="wrap">
  <button class="indicator" onclick={() => proxyPanelOpen.update((v) => !v)}>
    <div class="dot" class:active></div>
    <span class="name">{active?.name ?? $tr("proxy.noProxy")}</span>
    <Menu size={14} color="var(--text-3)" />
  </button>

  {#if $proxyPanelOpen}
    <ProxyPanel />
  {/if}
</div>

<style>
  .wrap {
    position: relative;
    flex: 0 1 auto;
    min-width: 0;
  }
  .indicator {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 10px;
    border-radius: 8px;
    background: var(--bg-3);
    border: 1px solid var(--border);
    color: var(--text-2);
    font-size: 13px;
    transition: background 0.12s, color 0.12s;
    max-width: min(180px, 30vw);
    min-width: 0;
  }
  .indicator:hover {
    background: var(--bg-4);
    color: var(--text);
  }
  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--border);
    flex-shrink: 0;
  }
  .dot.active {
    background: oklch(62% 0.18 145);
  }
  .name {
    max-width: 100px;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
