<script lang="ts">
  import { settings } from "$lib/stores/settings";
  import { LANGUAGE_OPTIONS, tr } from "$lib/i18n";
  import type { Language } from "$lib/types/settings";
  import Section from "./Section.svelte";
  import Row from "./Row.svelte";
  import Segmented from "./Segmented.svelte";
  import Toggle from "./Toggle.svelte";

  async function setLanguage(v: Language) {
    await settings.patch({ language: v });
  }
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

<Section title={$tr("general.interface")}>
  <Row label={$tr("general.language")} hint={$tr("general.languageHint")}>
    <Segmented
      value={$settings.language}
      onChange={setLanguage}
      options={LANGUAGE_OPTIONS}
    />
  </Row>
</Section>

<Section title={$tr("general.identity")}>
  <Row label={$tr("general.userName")} hint={$tr("general.userNameHint")}>
    <input
      class="text-input"
      value={$settings.user_name}
      oninput={setUserName}
      placeholder="User"
    />
  </Row>
  <Row label={$tr("general.assistantName")} hint={$tr("general.assistantNameHint")}>
    <input
      class="text-input"
      value={$settings.assistant_name}
      oninput={setAssistantName}
      placeholder="Scarlet"
    />
  </Row>
</Section>

<Section title={$tr("general.generation")}>
  <Row label={$tr("general.contextWindow")} hint={$tr("general.tokens")}>
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
  <Row label={$tr("general.maxResponse")} hint={$tr("general.tokens")}>
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
  <Row label={$tr("general.streaming")}>
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
