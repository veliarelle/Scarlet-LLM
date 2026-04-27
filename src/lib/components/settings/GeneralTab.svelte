<script lang="ts">
  import { settings } from "$lib/stores/settings";
  import Section from "./Section.svelte";
  import Row from "./Row.svelte";
  import Toggle from "./Toggle.svelte";

  async function setUserName(e: Event) {
    await settings.patch({ user_name: (e.target as HTMLInputElement).value });
  }
  async function setAssistantName(e: Event) {
    await settings.patch({ assistant_name: (e.target as HTMLInputElement).value });
  }
  async function setContextWindow(e: Event) {
    const v = Number((e.target as HTMLInputElement).value);
    if (!isNaN(v)) await settings.patch({ context_window: v });
  }
  async function setMaxTokens(e: Event) {
    const v = Number((e.target as HTMLInputElement).value);
    if (!isNaN(v)) await settings.patch({ max_tokens: v });
  }
  async function setStreaming(v: boolean) {
    await settings.patch({ streaming: v });
  }
</script>

<Section title="Identity">
  <Row label="Ваше имя" hint="label у user-сообщений">
    <input
      class="text-input"
      value={$settings.user_name}
      oninput={setUserName}
      placeholder="User"
    />
  </Row>
  <Row label="Имя ИИ" hint="label у assistant-сообщений">
    <input
      class="text-input"
      value={$settings.assistant_name}
      oninput={setAssistantName}
      placeholder="Scarlet"
    />
  </Row>
</Section>

<Section title="Generation">
  <Row label="Окно контекста" hint="токены">
    <input
      type="number"
      class="num-input"
      min="512"
      max="200000"
      step="512"
      value={$settings.context_window}
      onchange={setContextWindow}
    />
  </Row>
  <Row label="Макс. размер ответа" hint="токены">
    <input
      type="number"
      class="num-input"
      min="64"
      max="32000"
      step="64"
      value={$settings.max_tokens}
      onchange={setMaxTokens}
    />
  </Row>
  <Row label="Streaming">
    <Toggle value={$settings.streaming} onChange={setStreaming} />
  </Row>
</Section>

<style>
  .text-input {
    background: var(--bg-3);
    border: 1px solid var(--border);
    border-radius: 7px;
    padding: 6px 10px;
    color: var(--text);
    width: 200px;
  }
  .text-input:focus {
    border-color: var(--accent-d);
  }
  .num-input {
    width: 110px;
    background: var(--bg-3);
    border: 1px solid var(--border);
    border-radius: 7px;
    padding: 5px 10px;
    text-align: right;
    color: var(--text);
  }
  .num-input:focus {
    border-color: var(--accent-d);
  }
</style>
