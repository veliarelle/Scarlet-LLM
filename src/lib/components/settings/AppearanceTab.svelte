<script lang="ts">
  import { onMount } from "svelte";
  import { Image as ImageIcon, Trash2, RotateCcw } from "lucide-svelte";
  import { tr } from "$lib/i18n";
  import { settings } from "$lib/stores/settings";
  import { COLOR_VARS, type ColorVar, type Theme } from "$lib/types/settings";
  import {
    applyBackground,
    clearBgImage,
    readBg,
    writeBgBlur,
    writeBgDim,
    writeBgImage,
    type BgState,
  } from "$lib/utils/background";
  import Section from "./Section.svelte";
  import Row from "./Row.svelte";
  import Segmented from "./Segmented.svelte";
  import Toggle from "./Toggle.svelte";
  import "./AppearanceTab.css";

  let fileInput: HTMLInputElement | undefined = $state();
  let bg = $state<BgState>({ hasBg: false, dataUrl: "", blur: 0, dim: 0.4 });

  onMount(() => {
    bg = readBg();
  });

  // Дефолтные hex-эквиваленты для color-picker'a
  const DARK_HEX: Record<ColorVar, string> = {
    bg: "#170a08",
    "bg-2": "#1f0f0c",
    "bg-3": "#2a1612",
    "bg-4": "#3a1f1a",
    border: "#4a2924",
    accent: "#c43c45",
    "accent-h": "#d54d56",
    "accent-d": "#a02e34",
    text: "#efe6e2",
    "text-2": "#aea49f",
    "text-3": "#6b5e58",
    danger: "#d04444",
    highlight: "#e5c94a",
  };
  const LIGHT_HEX: Record<ColorVar, string> = {
    bg: "#f5efed",
    "bg-2": "#ece5e2",
    "bg-3": "#e3d9d4",
    "bg-4": "#d6cac4",
    border: "#bdb0a9",
    accent: "#b73843",
    "accent-h": "#c34250",
    "accent-d": "#9c2d36",
    text: "#241612",
    "text-2": "#52443f",
    "text-3": "#7d6e69",
    danger: "#b03039",
    highlight: "#e5c94a",
  };

  async function setTheme(v: Theme) {
    await settings.patch({ theme: v });
  }
  async function setUiScale(e: Event) {
    await settings.patch({ ui_scale: Number((e.target as HTMLInputElement).value) });
  }
  async function setTextScale(e: Event) {
    await settings.patch({ text_scale: Number((e.target as HTMLInputElement).value) });
  }
  async function setAssistantBubbles(v: boolean) {
    await settings.patch({ assistant_bubbles: v });
  }
  async function setUserBubbles(v: boolean) {
    await settings.patch({ user_bubbles: v });
  }

  async function setColor(v: ColorVar, e: Event) {
    const value = (e.target as HTMLInputElement).value;
    const next = { ...$settings.custom_colors, [v]: value };
    await settings.patch({ custom_colors: next });
  }
  async function resetColor(v: ColorVar) {
    const next = { ...$settings.custom_colors };
    delete next[v];
    await settings.patch({ custom_colors: next });
  }
  async function resetAllColors() {
    await settings.patch({ custom_colors: {} });
  }

  function colorValue(v: ColorVar): string {
    if ($settings.custom_colors[v]) return $settings.custom_colors[v];
    if ($settings.theme === "light") return LIGHT_HEX[v];
    return DARK_HEX[v];
  }

  const colorSuggestions = $derived(
    Array.from(
      new Set(
        Object.values($settings.custom_colors)
          .filter((value) => /^#[0-9a-fA-F]{6}$/.test(value))
          .map((value) => value.toLowerCase())
      )
    )
  );

  function uploadBg(e: Event) {
    const f = (e.target as HTMLInputElement).files?.[0];
    if (!f) return;
    const reader = new FileReader();
    reader.onload = (ev) => {
      const data = ev.target?.result as string;
      try {
        writeBgImage(data, $tr("appearance.bgTooLarge"));
        bg = { ...bg, dataUrl: data, hasBg: true };
        applyBackground(bg);
      } catch (err) {
        alert(String(err));
      }
    };
    reader.readAsDataURL(f);
    if (fileInput) fileInput.value = "";
  }

  function removeBg() {
    clearBgImage();
    bg = { hasBg: false, dataUrl: "", blur: 0, dim: 0.4 };
    applyBackground(bg);
  }

  function setBlur(e: Event) {
    bg = { ...bg, blur: Number((e.target as HTMLInputElement).value) };
    writeBgBlur(bg.blur);
    applyBackground(bg);
  }
  function setDim(e: Event) {
    bg = { ...bg, dim: Number((e.target as HTMLInputElement).value) };
    writeBgDim(bg.dim);
    applyBackground(bg);
  }

  // Translucency
  async function setSidebarTranslucent(v: boolean) {
    await settings.patch({ translucent_sidebar: v });
  }
  async function setSidebarBlur(e: Event) {
    await settings.patch({ sidebar_blur: Number((e.target as HTMLInputElement).value) });
  }
  async function setTopbarTranslucent(v: boolean) {
    await settings.patch({ translucent_topbar: v });
  }
  async function setTopbarBlur(e: Event) {
    await settings.patch({ topbar_blur: Number((e.target as HTMLInputElement).value) });
  }

  const VAR_LABELS = $derived<Record<ColorVar, string>>({
    bg: $tr("appearance.bgMain"),
    "bg-2": $tr("appearance.bgSidebar"),
    "bg-3": $tr("appearance.bgCards"),
    "bg-4": $tr("appearance.hoverBg"),
    border: $tr("appearance.border"),
    accent: $tr("appearance.accent"),
    "accent-h": $tr("appearance.accentHover"),
    "accent-d": $tr("appearance.accentDark"),
    text: $tr("appearance.mainText"),
    "text-2": $tr("appearance.textSecond"),
    "text-3": $tr("appearance.placeholderText"),
    danger: $tr("appearance.danger"),
    highlight: $tr("appearance.highlight"),
  });
</script>

<div class="appearance-tab">
  <Section title={$tr("appearance.theme")}>
    <Row label={$tr("appearance.colorScheme")}>
      <Segmented
        value={$settings.theme}
        onChange={setTheme}
        options={[
          { value: "dark", label: "🌙 Dark" },
          { value: "light", label: "☀️ Light" },
          { value: "custom", label: "🎨 Custom" },
        ]}
      />
    </Row>
    <Row label={$tr("appearance.uiScale")} hint="{$settings.ui_scale.toFixed(2)}x">
      <input
        type="range"
        class="range"
        min="0.75"
        max="1.5"
        step="0.05"
        value={$settings.ui_scale}
        oninput={setUiScale}
      />
    </Row>
    <Row label={$tr("appearance.textScale")} hint="{($settings.text_scale ?? 1).toFixed(2)}x">
      <input
        type="range"
        class="range"
        min="0.75"
        max="1.5"
        step="0.05"
        value={$settings.text_scale ?? 1}
        oninput={setTextScale}
      />
    </Row>
    <Row label={$tr("appearance.assistantBubbles")}>
      <Toggle
        value={$settings.assistant_bubbles ?? true}
        onChange={setAssistantBubbles}
        label={$tr("appearance.assistantBubblesHint")}
      />
    </Row>
    <Row label={$tr("appearance.userBubbles")}>
      <Toggle
        value={$settings.user_bubbles ?? true}
        onChange={setUserBubbles}
        label={$tr("appearance.userBubblesHint")}
      />
    </Row>
  </Section>

  {#if $settings.theme === "custom"}
    <Section title={$tr("appearance.customColors")}>
      <p class="hint">
        {$tr("appearance.customHint")}
      </p>
      <datalist id="scarlet-color-suggestions">
        {#each colorSuggestions as value (value)}
          <option value={value}></option>
        {/each}
      </datalist>
      <div class="color-grid">
        {#each COLOR_VARS as v (v)}
          <div class="color-row">
            <input
              type="color"
              class="picker"
              list="scarlet-color-suggestions"
              value={colorValue(v)}
              oninput={(e) => setColor(v, e)}
              aria-label={VAR_LABELS[v]}
            />
            <span class="var-name">--{v}</span>
            <span class="var-label">{VAR_LABELS[v]}</span>
            {#if $settings.custom_colors[v]}
              <button class="row-reset" onclick={() => resetColor(v)} aria-label={$tr("common.reset")}>
                <RotateCcw size={12} />
              </button>
            {/if}
          </div>
        {/each}
      </div>
      <button class="reset-btn" onclick={resetAllColors}>
        <RotateCcw size={14} /> {$tr("appearance.resetAllColors")}
      </button>
    </Section>
  {/if}

  <Section title={$tr("appearance.background")}>
    <div class="bg-preview" style:background-image={bg.hasBg ? `url(${bg.dataUrl})` : "none"}>
      {#if !bg.hasBg}
        <ImageIcon size={28} color="var(--text-3)" />
      {/if}
    </div>

    <div class="bg-actions">
      <button class="action-btn" onclick={() => fileInput?.click()}>
        <ImageIcon size={14} />
        {bg.hasBg ? $tr("appearance.change") : $tr("appearance.upload")}
      </button>
      {#if bg.hasBg}
        <button class="action-btn" onclick={removeBg}>
          <Trash2 size={14} /> {$tr("appearance.deleteBg")}
        </button>
      {/if}
      <input type="file" accept="image/*" bind:this={fileInput} onchange={uploadBg} hidden />
    </div>

    {#if bg.hasBg}
      <Row label="Blur" hint="{bg.blur}px">
        <input type="range" class="range" min="0" max="40" step="1" value={bg.blur} oninput={setBlur} />
      </Row>
      <Row label="Dim" hint="{Math.round(bg.dim * 100)}%">
        <input type="range" class="range" min="0" max="0.95" step="0.05" value={bg.dim} oninput={setDim} />
      </Row>
    {/if}
  </Section>

  <Section title={$tr("appearance.translucency")}>
    <p class="hint">
      {$tr("appearance.translucencyHint")}
    </p>
    <Row label={$tr("appearance.sidebar")}>
      <Toggle
        value={$settings.translucent_sidebar}
        onChange={setSidebarTranslucent}
        label={$tr("appearance.translucentSidebar")}
      />
    </Row>
    {#if $settings.translucent_sidebar}
      <Row label={$tr("appearance.blurStrength")} hint="{$settings.sidebar_blur}px">
        <input
          type="range"
          class="range"
          min="0"
          max="40"
          step="1"
          value={$settings.sidebar_blur}
          oninput={setSidebarBlur}
        />
      </Row>
    {/if}
    <Row label={$tr("appearance.topbar")}>
      <Toggle
        value={$settings.translucent_topbar}
        onChange={setTopbarTranslucent}
        label={$tr("appearance.translucentTopbar")}
      />
    </Row>
    {#if $settings.translucent_topbar}
      <Row label={$tr("appearance.blurStrength")} hint="{$settings.topbar_blur}px">
        <input
          type="range"
          class="range"
          min="0"
          max="40"
          step="1"
          value={$settings.topbar_blur}
          oninput={setTopbarBlur}
        />
      </Row>
    {/if}
  </Section>
</div>
