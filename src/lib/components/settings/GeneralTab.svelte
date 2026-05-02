<script lang="ts">
  import { Download, Upload } from "lucide-svelte";
  import { api } from "$lib/api/invoke";
  import { settings } from "$lib/stores/settings";
  import { refreshPresets } from "$lib/stores/presets";
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
  async function setShowUserName(v: boolean) {
    await settings.patch({ show_user_name: v });
  }
  async function setAssistantName(e: Event) {
    await settings.patch({ assistant_name: (e.target as HTMLInputElement).value });
  }
  async function setShowAssistantName(v: boolean) {
    await settings.patch({ show_assistant_name: v });
  }
  async function setContextWindow(e: Event) {
    const v = Number((e.target as HTMLInputElement).value);
    if (!isNaN(v)) await settings.patch({ context_window: v });
  }
  async function setMaxTokens(e: Event) {
    const v = Number((e.target as HTMLInputElement).value);
    if (!isNaN(v)) await settings.patch({ max_tokens: v });
  }
  async function setMaxMessageSize(e: Event) {
    const v = Number((e.target as HTMLInputElement).value);
    if (!isNaN(v)) await settings.patch({ max_message_size: v });
  }
  async function setStreaming(v: boolean) {
    await settings.patch({ streaming: v });
  }
  async function setShowTokenCounts(v: boolean) {
    await settings.patch({ show_token_counts: v });
  }
  async function setShowMessageModels(v: boolean) {
    await settings.patch({ show_message_models: v });
  }
  async function setShowMessageTime(v: boolean) {
    await settings.patch({ show_message_time: v });
  }
  async function setPromptCaching(v: boolean) {
    await settings.patch({ prompt_caching: v });
  }
  async function exportProfile() {
    await api.exportProfile();
  }
  async function importProfile() {
    if (!confirm($tr("general.importProfileConfirm"))) return;
    const count = await api.importProfile();
    if (count > 0) await refreshPresets();
    await settings.load();
  }
</script>

<Section title={$tr("general.profile")}>
  <p class="hint">{$tr("general.profileHint")}</p>
  <div class="profile-actions">
    <button class="action-btn" onclick={exportProfile}>
      <Download size={14} /> {$tr("general.exportProfile")}
    </button>
    <button class="action-btn" onclick={importProfile}>
      <Upload size={14} /> {$tr("general.importProfile")}
    </button>
  </div>
</Section>

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
  <Row label={$tr("general.showUserName")}>
    <Toggle value={$settings.show_user_name ?? true} onChange={setShowUserName} />
  </Row>
  <Row label={$tr("general.assistantName")} hint={$tr("general.assistantNameHint")}>
    <input
      class="text-input"
      value={$settings.assistant_name}
      oninput={setAssistantName}
      placeholder="Scarlet"
    />
  </Row>
  <Row label={$tr("general.showAssistantName")}>
    <Toggle value={$settings.show_assistant_name ?? true} onChange={setShowAssistantName} />
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
  <Row label={$tr("general.maxMessageSize")} hint={$tr("general.characters")}>
    <input
      type="number"
      class="num-input"
      min="0"
      max="1000000"
      step="1000"
      value={$settings.max_message_size}
      onchange={setMaxMessageSize}
    />
  </Row>
  <Row label={$tr("general.streaming")}>
    <Toggle value={$settings.streaming} onChange={setStreaming} />
  </Row>
  <Row label={$tr("general.showTokenCounts")} hint={$tr("general.showTokenCountsHint")}>
    <Toggle value={$settings.show_token_counts} onChange={setShowTokenCounts} />
  </Row>
  <Row label={$tr("general.showMessageModels")} hint={$tr("general.showMessageModelsHint")}>
    <Toggle value={$settings.show_message_models ?? true} onChange={setShowMessageModels} />
  </Row>
  <Row label={$tr("general.showMessageTime")} hint={$tr("general.showMessageTimeHint")}>
    <Toggle value={$settings.show_message_time ?? true} onChange={setShowMessageTime} />
  </Row>
  <Row label={$tr("general.promptCaching")} hint={$tr("general.promptCachingHint")}>
    <Toggle value={$settings.prompt_caching} onChange={setPromptCaching} />
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
  .hint {
    font-size: 0.733rem;
    color: var(--text-3);
    line-height: 1.4;
  }
  .profile-actions {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }
  .action-btn {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px;
    border-radius: 7px;
    font-size: 0.8rem;
    background: var(--bg-3);
    border: 1px dashed var(--border);
    color: var(--text-3);
    transition: background 0.12s, color 0.12s;
  }
  .action-btn:hover {
    background: var(--bg-4);
    color: var(--text-2);
  }
</style>
