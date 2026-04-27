<script lang="ts">
  let {
    value,
    onChange,
    label = "Toggle",
  }: { value: boolean; onChange: (v: boolean) => void; label?: string } = $props();
</script>

<button
  class="toggle"
  class:on={value}
  onclick={() => onChange(!value)}
  aria-pressed={value}
  aria-label={label}
  type="button"
>
  <span class="thumb"></span>
</button>

<style>
  .toggle {
    width: 38px;
    height: 22px;
    border-radius: 11px;
    background: var(--bg-4);
    border: 1px solid var(--border);
    position: relative;
    transition: background 0.2s, border-color 0.2s;
    flex-shrink: 0;
    padding: 0;
  }
  .toggle.on {
    background: var(--accent);
    border-color: var(--accent);
  }
  .thumb {
    position: absolute;
    top: 2px;
    left: 2px;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: var(--text-3);
    transition: transform 0.2s, background 0.2s;
  }
  .toggle.on .thumb {
    transform: translateX(16px);
    background: white;
  }
</style>
