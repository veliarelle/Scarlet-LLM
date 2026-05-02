<script lang="ts">
  import type { Snippet } from "svelte";

  let {
    label,
    hint,
    children,
  }: {
    label: string;
    hint?: string;
    children: Snippet;
  } = $props();
</script>

<div class="row">
  <div class="row-label">
    <span>{label}</span>
    {#if hint}<span class="row-hint">{hint}</span>{/if}
  </div>
  <div class="row-control">
    {@render children()}
  </div>
</div>

<style>
  .row {
    display: flex;
    align-items: center;
    gap: 12px;
    min-height: 34px;
  }
  .row-label {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }
  .row-hint {
    font-size: 0.733rem;
    color: var(--text-3);
  }
  .row-control {
    flex-shrink: 0;
  }
</style>
