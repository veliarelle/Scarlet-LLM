<script lang="ts" generics="T extends string">
  let {
    value,
    onChange,
    options,
  }: {
    value: T;
    onChange: (v: T) => void;
    options: { value: T; label: string }[];
  } = $props();
</script>

<div class="segmented">
  {#each options as opt (opt.value)}
    <button
      class="seg-btn"
      class:active={value === opt.value}
      onclick={() => onChange(opt.value)}
      type="button"
    >
      {opt.label}
    </button>
  {/each}
</div>

<style>
  .segmented {
    display: flex;
    background: var(--bg-3);
    border-radius: 8px;
    padding: 2px;
    gap: 2px;
    border: 1px solid var(--border);
  }
  .seg-btn {
    padding: 4px 12px;
    border-radius: 6px;
    font-size: 0.8rem;
    color: var(--text-3);
    transition: background 0.12s, color 0.12s;
  }
  .seg-btn.active {
    background: var(--bg-4);
    color: var(--text);
  }
</style>
