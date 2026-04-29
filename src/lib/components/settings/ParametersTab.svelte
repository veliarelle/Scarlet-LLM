<script lang="ts">
  import { ListPlus, Plus, Trash2, X } from "lucide-svelte";
  import { tr } from "$lib/i18n";
  import { settings } from "$lib/stores/settings";
  import { uid } from "$lib/utils/id";
  import type { ParamEntry } from "$lib/types/settings";
  import Section from "./Section.svelte";
  import Toggle from "./Toggle.svelte";

  type ParamPreset = {
    group: string;
    key: string;
    value: string;
    note: string;
  };

  const PARAM_PRESETS: ParamPreset[] = [
    { group: "Core sampling", key: "temperature", value: "0.7", note: "OpenAI, Anthropic, Gemini, OpenRouter, most local models" },
    { group: "Core sampling", key: "top_p", value: "0.9", note: "Nucleus sampling" },
    { group: "Core sampling", key: "top_k", value: "40", note: "Gemini, Anthropic, many local models" },
    { group: "Core sampling", key: "min_p", value: "0.05", note: "OpenRouter/local samplers" },
    { group: "Core sampling", key: "typical_p", value: "0.95", note: "Local/OpenRouter-style samplers" },
    { group: "Core sampling", key: "top_a", value: "0.0", note: "OpenRouter-style sampler" },
    { group: "Core sampling", key: "epsilon_cutoff", value: "0", note: "Local samplers" },
    { group: "Core sampling", key: "eta_cutoff", value: "0", note: "Local samplers" },
    { group: "Repetition", key: "frequency_penalty", value: "0", note: "OpenAI-compatible" },
    { group: "Repetition", key: "presence_penalty", value: "0", note: "OpenAI-compatible" },
    { group: "Repetition", key: "repetition_penalty", value: "1.05", note: "Local/OpenRouter-style models" },
    { group: "Repetition", key: "repetition_penalty_range", value: "1024", note: "Local samplers" },
    { group: "Repetition", key: "encoder_repetition_penalty", value: "1.0", note: "Local samplers" },
    { group: "Limits / stops", key: "max_completion_tokens", value: "2048", note: "OpenAI-compatible completion cap" },
    { group: "Limits / stops", key: "max_output_tokens", value: "2048", note: "Responses/Gemini-style output cap" },
    { group: "Limits / stops", key: "stop", value: "[\"\\nUser:\"]", note: "OpenAI-compatible stop strings" },
    { group: "Limits / stops", key: "stop_sequences", value: "[\"\\nUser:\"]", note: "Anthropic/Gemini-style stop strings" },
    { group: "Determinism", key: "seed", value: "42", note: "OpenAI-compatible/OpenRouter/local when supported" },
    { group: "Determinism", key: "random_seed", value: "42", note: "Mistral/local variants" },
    { group: "Structured output", key: "response_format", value: "{\"type\":\"json_object\"}", note: "OpenAI-compatible JSON mode" },
    { group: "Structured output", key: "json_schema", value: "{\"name\":\"schema\",\"schema\":{\"type\":\"object\"}}", note: "Schema-style outputs" },
    { group: "Tools", key: "parallel_tool_calls", value: "true", note: "OpenAI-compatible tools" },
    { group: "Tools", key: "tool_choice", value: "auto", note: "OpenAI-compatible tools" },
    { group: "OpenAI-compatible", key: "logit_bias", value: "{}", note: "Bias specific token ids" },
    { group: "OpenAI-compatible", key: "stream_options", value: "{\"include_usage\":true}", note: "Usage details for streaming" },
    { group: "OpenAI-compatible", key: "modalities", value: "[\"text\"]", note: "Text/audio multimodal response control" },
    { group: "OpenAI-compatible", key: "prediction", value: "{\"type\":\"content\",\"content\":\"\"}", note: "Predicted output hint where supported" },
    { group: "OpenAI / Responses", key: "reasoning_effort", value: "medium", note: "Reasoning models; duplicates Reasoning tab if enabled" },
    { group: "OpenAI / Responses", key: "verbosity", value: "medium", note: "GPT-5 style response verbosity where supported" },
    { group: "OpenAI / Responses", key: "service_tier", value: "auto", note: "OpenAI service tier" },
    { group: "OpenAI / Responses", key: "logprobs", value: "false", note: "Token log probabilities" },
    { group: "OpenAI / Responses", key: "top_logprobs", value: "0", note: "Token log probabilities count" },
    { group: "Anthropic", key: "thinking", value: "{\"type\":\"enabled\",\"budget_tokens\":1024}", note: "Extended thinking where supported" },
    { group: "Anthropic", key: "metadata", value: "{}", note: "Request metadata" },
    { group: "Mistral / Cohere", key: "safe_prompt", value: "false", note: "Mistral safety preprompt toggle" },
    { group: "Mistral / Cohere", key: "random_seed", value: "42", note: "Mistral deterministic seed" },
    { group: "Mistral / Cohere", key: "k", value: "40", note: "Cohere top-k" },
    { group: "Mistral / Cohere", key: "p", value: "0.9", note: "Cohere nucleus sampling" },
    { group: "Mistral / Cohere", key: "return_likelihoods", value: "NONE", note: "Cohere likelihood output" },
    { group: "Gemini / Google", key: "candidate_count", value: "1", note: "Gemini candidate count" },
    { group: "Gemini / Google", key: "response_mime_type", value: "application/json", note: "Gemini structured MIME output" },
    { group: "Gemini / Google", key: "response_schema", value: "{\"type\":\"object\"}", note: "Gemini response schema" },
    { group: "Gemini / Google", key: "safety_settings", value: "[]", note: "Gemini safety settings array" },
    { group: "OpenRouter", key: "models", value: "[]", note: "OpenRouter fallback model list" },
    { group: "OpenRouter", key: "route", value: "fallback", note: "OpenRouter routing" },
    { group: "OpenRouter", key: "provider", value: "{\"allow_fallbacks\":true}", note: "OpenRouter provider preferences" },
    { group: "Ollama / llama.cpp", key: "num_ctx", value: "8192", note: "Context window" },
    { group: "Ollama / llama.cpp", key: "num_predict", value: "2048", note: "Output token limit" },
    { group: "Ollama / llama.cpp", key: "repeat_last_n", value: "64", note: "Repeat penalty lookback" },
    { group: "Ollama / llama.cpp", key: "repeat_penalty", value: "1.1", note: "Repeat penalty" },
    { group: "Ollama / llama.cpp", key: "mirostat", value: "0", note: "Mirostat mode: 0/1/2" },
    { group: "Ollama / llama.cpp", key: "mirostat_tau", value: "5.0", note: "Mirostat target entropy" },
    { group: "Ollama / llama.cpp", key: "mirostat_eta", value: "0.1", note: "Mirostat learning rate" },
    { group: "Ollama / llama.cpp", key: "tfs_z", value: "1.0", note: "Tail-free sampling" },
    { group: "Ollama / llama.cpp", key: "grammar", value: "", note: "llama.cpp grammar constraint" },
    { group: "Kobold / Textgen", key: "max_new_tokens", value: "2048", note: "Text Generation WebUI/Kobold output cap" },
    { group: "Kobold / Textgen", key: "do_sample", value: "true", note: "Transformers sampling toggle" },
    { group: "Kobold / Textgen", key: "penalty_alpha", value: "0", note: "Contrastive search" },
    { group: "Kobold / Textgen", key: "length_penalty", value: "1.0", note: "Beam/search length penalty" },
    { group: "Kobold / Textgen", key: "early_stopping", value: "false", note: "Beam search early stop" },
    { group: "vLLM / TGI", key: "best_of", value: "1", note: "Server-side candidate count" },
    { group: "vLLM / TGI", key: "use_beam_search", value: "false", note: "vLLM beam search" },
    { group: "vLLM / TGI", key: "ignore_eos", value: "false", note: "Continue past EOS where supported" },
    { group: "vLLM / TGI", key: "watermark", value: "false", note: "TGI watermark toggle" },
    { group: "Image generation", key: "size", value: "1024x1024", note: "OpenAI image size" },
    { group: "Image generation", key: "resolution", value: "1024x1024", note: "Proxy/model-specific resolution" },
    { group: "Image generation", key: "image_size", value: "1024x1024", note: "OpenRouter/image model variants" },
    { group: "Image generation", key: "aspect_ratio", value: "1:1", note: "Gemini/OpenRouter image ratios" },
    { group: "Image generation", key: "width", value: "1024", note: "Diffusion/OpenRouter image width" },
    { group: "Image generation", key: "height", value: "1024", note: "Diffusion/OpenRouter image height" },
    { group: "Image generation", key: "quality", value: "auto", note: "OpenAI image quality" },
    { group: "Image generation", key: "background", value: "auto", note: "OpenAI image background" },
    { group: "Image generation", key: "output_format", value: "png", note: "Image output format" },
    { group: "Image generation", key: "output_compression", value: "100", note: "Image compression where supported" },
    { group: "Image generation", key: "steps", value: "30", note: "Diffusion step count" },
    { group: "Image generation", key: "cfg_scale", value: "7", note: "Diffusion prompt guidance" },
    { group: "Image generation", key: "negative_prompt", value: "", note: "Diffusion negative prompt" },
    { group: "Image generation", key: "style", value: "natural", note: "Image style where supported" },
    { group: "Image generation", key: "n", value: "1", note: "Number of images/candidates where supported" },
  ];

  let listOpen = $state(false);
  let presetSearch = $state("");

  const filteredPresets = $derived(PARAM_PRESETS.filter((preset) => {
    const q = presetSearch.trim().toLowerCase();
    if (!q) return true;
    return `${preset.group} ${preset.key} ${preset.value} ${preset.note}`.toLowerCase().includes(q);
  }));
  const presetGroups = $derived(Array.from(new Set(filteredPresets.map((preset) => preset.group))));

  async function setParams(next: ParamEntry[]) {
    await settings.patch({ params: next });
  }

  async function addParam() {
    await setParams([
      ...$settings.params,
      { id: uid(), key: "", value: "", enabled: true },
    ]);
  }

  async function addFromJson() {
    const json = prompt($tr("params.jsonPrompt", { example: '{"my_param": 0.5, "stop": ["<END>"]}' }));
    if (!json) return;
    try {
      const obj = JSON.parse(json);
      if (obj === null || typeof obj !== "object" || Array.isArray(obj)) {
        alert($tr("params.objectExpected"));
        return;
      }
      const entries: ParamEntry[] = Object.entries(obj).map(([k, v]) => ({
        id: uid(),
        key: k,
        value: typeof v === "string" ? v : JSON.stringify(v),
        enabled: true,
      }));
      await setParams([...$settings.params, ...entries]);
    } catch {
      alert($tr("params.invalidJson"));
    }
  }

  async function addPreset(preset: ParamPreset) {
    await setParams([
      ...$settings.params,
      { id: uid(), key: preset.key, value: preset.value, enabled: true },
    ]);
  }

  async function update(id: string, patch: Partial<ParamEntry>) {
    await setParams($settings.params.map((p) => (p.id === id ? { ...p, ...patch } : p)));
  }

  async function remove(id: string) {
    await setParams($settings.params.filter((p) => p.id !== id));
  }
</script>

<Section title={$tr("params.section")}>
  <div class="params">
    {#each $settings.params as p (p.id)}
      <div class="row">
        <Toggle value={p.enabled} onChange={(v) => update(p.id, { enabled: v })} />
        <input
          class="text-input key"
          value={p.key}
          oninput={(e) => update(p.id, { key: (e.target as HTMLInputElement).value })}
          placeholder="key"
        />
        <input
          class="text-input val"
          value={p.value}
          oninput={(e) => update(p.id, { value: (e.target as HTMLInputElement).value })}
          placeholder="value"
        />
        <button class="del-btn" onclick={() => remove(p.id)} aria-label={$tr("params.delete")}>
          <Trash2 size={14} color="var(--text-3)" />
        </button>
      </div>
    {/each}
    <div class="actions">
      <button class="add-btn" onclick={addParam}>
        <Plus size={14} /> {$tr("params.add")}
      </button>
      <button class="add-btn" onclick={addFromJson}>
        <Plus size={14} /> {$tr("params.addFromJson")}
      </button>
      <button class="add-btn" onclick={() => (listOpen = !listOpen)}>
        <ListPlus size={14} /> {$tr("params.addFromList")}
      </button>
    </div>

    {#if listOpen}
      <div class="preset-panel">
        <div class="preset-head">
          <input
            class="text-input preset-search"
            bind:value={presetSearch}
            placeholder={$tr("params.searchPresets")}
          />
          <button class="close-btn" onclick={() => (listOpen = false)} aria-label={$tr("common.close")}>
            <X size={14} />
          </button>
        </div>
        <div class="preset-list">
          {#each presetGroups as group (group)}
            <div class="preset-group">{group}</div>
            {#each filteredPresets.filter((preset) => preset.group === group) as preset (`${preset.group}:${preset.key}:${preset.value}`)}
              <button class="preset-item" onclick={() => addPreset(preset)}>
                <span class="preset-main">
                  <span class="preset-key">{preset.key}</span>
                  <span class="preset-value">{preset.value}</span>
                </span>
                <span class="preset-note">{preset.note}</span>
              </button>
            {/each}
          {/each}
          {#if filteredPresets.length === 0}
            <div class="preset-empty">{$tr("params.noPresetResults")}</div>
          {/if}
        </div>
      </div>
    {/if}
  </div>
</Section>

<style>
  .params {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 7px;
  }
  .text-input {
    background: var(--bg-3);
    border: 1px solid var(--border);
    border-radius: 7px;
    padding: 6px 10px;
    color: var(--text);
    font-size: 13px;
  }
  .text-input:focus {
    border-color: var(--accent-d);
  }
  .text-input.key {
    width: 130px;
    flex-shrink: 0;
    font-family: "JetBrains Mono", monospace;
  }
  .text-input.val {
    flex: 1;
    min-width: 0;
  }
  .del-btn {
    display: flex;
    align-items: center;
    opacity: 0.5;
  }
  .del-btn:hover {
    opacity: 1;
  }
  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin-top: 4px;
  }
  .add-btn {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px;
    border-radius: 7px;
    font-size: 12px;
    background: var(--bg-3);
    border: 1px dashed var(--border);
    color: var(--text-3);
    transition: background 0.12s, color 0.12s;
  }
  .add-btn:hover {
    background: var(--bg-4);
    color: var(--text-2);
  }
  .preset-panel {
    margin-top: 4px;
    background: var(--bg-2);
    border: 1px solid var(--border);
    border-radius: 10px;
    overflow: hidden;
  }
  .preset-head {
    display: flex;
    gap: 6px;
    padding: 8px;
    border-bottom: 1px solid var(--border);
  }
  .preset-search {
    flex: 1;
    min-width: 0;
    background: var(--bg-3);
  }
  .close-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    border-radius: 7px;
    color: var(--text-3);
  }
  .close-btn:hover {
    background: var(--bg-4);
    color: var(--text);
  }
  .preset-list {
    max-height: min(460px, 52vh);
    overflow-y: auto;
    padding: 6px;
  }
  .preset-group {
    padding: 8px 8px 4px;
    font-size: 11px;
    font-weight: 700;
    color: var(--text-3);
    text-transform: uppercase;
    letter-spacing: 0.08em;
  }
  .preset-item {
    display: flex;
    flex-direction: column;
    align-items: stretch;
    gap: 2px;
    width: 100%;
    padding: 7px 8px;
    border-radius: 7px;
    text-align: left;
    color: var(--text-2);
  }
  .preset-item:hover {
    background: var(--bg-3);
    color: var(--text);
  }
  .preset-main {
    display: flex;
    align-items: baseline;
    gap: 8px;
    min-width: 0;
  }
  .preset-key,
  .preset-value {
    font-family: "JetBrains Mono", monospace;
    font-size: 12px;
  }
  .preset-key {
    color: var(--text);
  }
  .preset-value {
    color: var(--text-3);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .preset-note,
  .preset-empty {
    color: var(--text-3);
    font-size: 11px;
  }
  .preset-empty {
    padding: 12px 8px;
  }
</style>
