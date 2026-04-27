<script lang="ts">
  import { renderMarkdown } from "$lib/utils/markdown";

  let { content }: { content: string } = $props();
  const html = $derived(renderMarkdown(content));
</script>

<div class="sl-md-content">
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
  .sl-md-content :global(.sl-codeblock) {
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 10px 14px;
    margin: 0.4em 0;
    overflow-x: auto;
    font-family: "JetBrains Mono", monospace;
    font-size: 0.85em;
    line-height: 1.55;
  }
</style>
