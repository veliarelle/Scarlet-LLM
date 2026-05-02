<script lang="ts">
  import type { Snippet } from "svelte";

  let { title, children }: { title?: string; children: Snippet } = $props();
</script>

<div class="section">
  {#if title}<div class="section-title">{title}</div>{/if}
  {@render children()}
</div>

<style>
  .section {
    display: flex;
    flex-direction: column;
    gap: 9px;
  }
  .section-title {
    font-size: 0.667rem;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    color: var(--text-3);
  }
</style>
