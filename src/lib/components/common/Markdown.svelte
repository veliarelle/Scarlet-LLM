<script lang="ts">
  import { renderMarkdown } from "$lib/utils/markdown";
  import { tr } from "$lib/i18n";

  let { content }: { content: string } = $props();
  const html = $derived(renderMarkdown(content, { copy: $tr("common.copy"), copied: $tr("common.copied") }));

  function copyFromEvent(e: Event) {
    const target = e.target as HTMLElement | null;
    const btn = target?.closest<HTMLButtonElement>(".sl-code-copy");
    if (!btn) return;
    const code = btn.closest(".sl-codewrap")?.querySelector("code")?.textContent ?? "";
    if (!code) return;
    const copyLabel = btn.dataset.copyLabel ?? $tr("common.copy");
    const copiedLabel = btn.dataset.copiedLabel ?? $tr("common.copied");
    navigator.clipboard.writeText(code).then(() => {
      btn.classList.add("copied");
      btn.setAttribute("aria-label", copiedLabel);
      btn.setAttribute("title", copiedLabel);
      window.setTimeout(() => {
        btn.classList.remove("copied");
        btn.setAttribute("aria-label", copyLabel);
        btn.setAttribute("title", copyLabel);
      }, 1200);
    }).catch(() => {});
  }

  function codeCopyAction(node: HTMLElement) {
    node.addEventListener("click", copyFromEvent);
    return {
      destroy() {
        node.removeEventListener("click", copyFromEvent);
      },
    };
  }
</script>

<div class="sl-md-content" use:codeCopyAction>
  {@html html}
</div>

<style>
  .sl-md-content {
    line-height: 1.65;
    word-break: break-word;
  }

  .sl-md-content :global(h1.sl-md-h1) {
    font-size: 1.4em;
    font-weight: 700;
    margin: 0.6em 0 0.3em;
  }
  .sl-md-content :global(h2.sl-md-h2) {
    font-size: 1.2em;
    font-weight: 600;
    margin: 0.5em 0 0.25em;
  }
  .sl-md-content :global(h3.sl-md-h3) {
    font-size: 1.05em;
    font-weight: 600;
    margin: 0.4em 0 0.2em;
  }
  .sl-md-content :global(p.sl-md-p) {
    margin: 0.2em 0;
  }
  .sl-md-content :global(.sl-md-br) {
    height: 0.5em;
  }
  .sl-md-content :global(ul.sl-md-ul),
  .sl-md-content :global(ol.sl-md-ol) {
    margin: 0.3em 0 0.3em 1.4em;
  }
  .sl-md-content :global(li) {
    margin: 0.15em 0;
  }
  .sl-md-content :global(blockquote.sl-md-bq) {
    border-left: 3px solid var(--accent-d);
    padding-left: 0.8em;
    margin: 0.4em 0;
    color: var(--text-2);
    font-style: italic;
  }
  .sl-md-content :global(hr.sl-md-hr) {
    border: none;
    border-top: 1px solid var(--border);
    margin: 0.6em 0;
  }
  .sl-md-content :global(a) {
    color: var(--accent);
  }
  .sl-md-content :global(strong) {
    font-weight: 600;
  }
  .sl-md-content :global(em) {
    font-style: italic;
  }
  .sl-md-content :global(.sl-icode) {
    background: var(--bg-4);
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 1px 5px;
    font-family: "JetBrains Mono", monospace;
    font-size: 0.88em;
  }
  .sl-md-content :global(.sl-codewrap) {
    position: relative;
    margin: 0.4em 0;
  }
  .sl-md-content :global(.sl-code-copy) {
    position: absolute;
    top: 7px;
    right: 7px;
    z-index: 1;
    width: 28px;
    height: 28px;
    padding: 0 8px;
    border-radius: 6px;
    border: 0;
    background: transparent;
    color: var(--text-2);
    line-height: 1;
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }
  .sl-md-content :global(.sl-code-copy:hover) {
    background: var(--bg-4);
    color: var(--text);
  }
  .sl-md-content :global(.sl-copy-icon) {
    position: relative;
    width: 14px;
    height: 14px;
    display: inline-block;
  }
  .sl-md-content :global(.sl-copy-icon::before),
  .sl-md-content :global(.sl-copy-icon::after) {
    content: "";
    position: absolute;
    width: 9px;
    height: 10px;
    border: 1.6px solid currentColor;
    border-radius: 2px;
  }
  .sl-md-content :global(.sl-copy-icon::before) {
    left: 1px;
    top: 1px;
    opacity: 0.55;
  }
  .sl-md-content :global(.sl-copy-icon::after) {
    right: 1px;
    bottom: 1px;
    background: var(--bg);
  }
  .sl-md-content :global(.sl-code-copy.copied .sl-copy-icon::before) {
    display: none;
  }
  .sl-md-content :global(.sl-code-copy.copied .sl-copy-icon::after) {
    width: 12px;
    height: 7px;
    right: 1px;
    bottom: 4px;
    border-top: 0;
    border-right: 0;
    border-radius: 0;
    background: transparent;
    transform: rotate(-45deg);
    color: var(--accent);
  }
  .sl-md-content :global(.sl-codeblock) {
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 28px 14px 10px;
    margin: 0;
    overflow-x: auto;
    font-family: "JetBrains Mono", monospace;
    font-size: 0.85em;
    line-height: 1.55;
  }
</style>
