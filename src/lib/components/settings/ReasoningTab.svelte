<script lang="ts">
  import { settings } from "$lib/stores/settings";
  import { tr } from "$lib/i18n";
  import Section from "./Section.svelte";
  import Row from "./Row.svelte";
  import Toggle from "./Toggle.svelte";
  import Segmented from "./Segmented.svelte";
  import type { ReasoningConfig } from "$lib/types/settings";

  async function patchR(p: Partial<ReasoningConfig>) {
    await settings.patch({ reasoning: { ...$settings.reasoning, ...p } });
  }
</script>

<Section title={$tr("reasoning.section")}>
  <Row label={$tr("reasoning.enabled")} hint={$tr("reasoning.enabledHint")}>
    <Toggle value={$settings.reasoning.enabled} onChange={(v) => patchR({ enabled: v })} />
  </Row>
  {#if $settings.reasoning.enabled}
    <Row label={$tr("reasoning.level")} hint="low/medium/high">
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
    <Row label={$tr("reasoning.sendEffort")} hint={$tr("reasoning.sendEffortHint")}>
      <Toggle
        value={$settings.reasoning.send_effort}
        onChange={(v) => patchR({ send_effort: v })}
      />
    </Row>
  {/if}
</Section>
