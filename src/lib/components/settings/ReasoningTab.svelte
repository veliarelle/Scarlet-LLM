<script lang="ts">
  import { settings } from "$lib/stores/settings";
  import Section from "./Section.svelte";
  import Row from "./Row.svelte";
  import Toggle from "./Toggle.svelte";
  import Segmented from "./Segmented.svelte";
  import type { ReasoningConfig } from "$lib/types/settings";

  async function patchR(p: Partial<ReasoningConfig>) {
    await settings.patch({ reasoning: { ...$settings.reasoning, ...p } });
  }
</script>

<Section title="Reasoning">
  <Row label="Включить reasoning" hint="отправлять reasoning_effort с запросом">
    <Toggle value={$settings.reasoning.enabled} onChange={(v) => patchR({ enabled: v })} />
  </Row>
  {#if $settings.reasoning.enabled}
    <Row label="Уровень" hint="low/medium/high">
      <Segmented
        value={$settings.reasoning.effort}
        onChange={(v) => patchR({ effort: v })}
        options={[
          { value: "low", label: "Low" },
          { value: "medium", label: "Med" },
          { value: "high", label: "High" },
        ]}
      />
    </Row>
    <Row label="Отправлять effort" hint="можно выкл, если модель не поддерживает">
      <Toggle
        value={$settings.reasoning.send_effort}
        onChange={(v) => patchR({ send_effort: v })}
      />
    </Row>
  {/if}
</Section>
