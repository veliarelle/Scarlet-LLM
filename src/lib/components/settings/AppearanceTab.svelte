<script lang="ts">
  import { onMount } from "svelte";
  import { Image as ImageIcon, Trash2, RotateCcw } from "lucide-svelte";
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
  };

  async function setTheme(v: Theme) {
    await settings.patch({ theme: v });
  }
  async function setUiScale(e: Event) {
    await settings.patch({ ui_scale: Number((e.target as HTMLInputElement).value) });
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

  function uploadBg(e: Event) {
    const f = (e.target as HTMLInputElement).files?.[0];
    if (!f) return;
    const reader = new FileReader();
    reader.onload = (ev) => {
      const data = ev.target?.result as string;
      try {
        writeBgImage(data);
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

  const VAR_LABELS: Record<ColorVar, string> = {
    bg: "Фон (основной)",
    "bg-2": "Фон сайдбара/drawer",
    "bg-3": "Фон карточек/инпутов",
    "bg-4": "Hover-фон",
    border: "Границы",
    accent: "Акцент",
    "accent-h": "Акцент (hover)",
    "accent-d": "Акцент (тёмный)",
    text: "Основной текст",
    "text-2": "Второй уровень текста",
    "text-3": "Подсказки/placeholder",
    danger: "Ошибка/удаление",
  };
</script>

<Section title="Theme">
  <Row label="Цветовая схема">
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
  <Row label="Размер UI" hint="{$settings.ui_scale.toFixed(2)}x">
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
</Section>

{#if $settings.theme === "custom"}
  <Section title="Custom colors">
    <p class="hint">
      Переопределённые цвета сохраняются в settings.json. Сброс возвращает встроенный dark.
    </p>
    <div class="color-grid">
      {#each COLOR_VARS as v (v)}
        <div class="color-row">
          <input
            type="color"
            class="picker"
            value={colorValue(v)}
            oninput={(e) => setColor(v, e)}
            aria-label={VAR_LABELS[v]}
          />
          <span class="var-name">--{v}</span>
          <span class="var-label">{VAR_LABELS[v]}</span>
          {#if $settings.custom_colors[v]}
            <button class="row-reset" onclick={() => resetColor(v)} aria-label="Сбросить">
              <RotateCcw size={12} />
            </button>
          {/if}
        </div>
      {/each}
    </div>
    <button class="reset-btn" onclick={resetAllColors}>
      <RotateCcw size={14} /> Сбросить все цвета
    </button>
  </Section>
{/if}

<Section title="Background">
  <div class="bg-preview" style:background-image={bg.hasBg ? `url(${bg.dataUrl})` : "none"}>
    {#if !bg.hasBg}
      <ImageIcon size={28} color="var(--text-3)" />
    {/if}
  </div>

  <div class="bg-actions">
    <button class="action-btn" onclick={() => fileInput?.click()}>
      <ImageIcon size={14} />
      {bg.hasBg ? "Сменить" : "Загрузить"}
    </button>
    {#if bg.hasBg}
      <button class="action-btn" onclick={removeBg}>
        <Trash2 size={14} /> Удалить
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

<Section title="Полупрозрачность">
  <p class="hint">
    Делает сайдбар или верхний бар полупрозрачным с блюром (видно фон под ними).
    Доступно с фоновым изображением или без.
  </p>
  <Row label="Сайдбар">
    <Toggle
      value={$settings.translucent_sidebar}
      onChange={setSidebarTranslucent}
      label="Полупрозрачный сайдбар"
    />
  </Row>
  {#if $settings.translucent_sidebar}
    <Row label="Сила блюра" hint="{$settings.sidebar_blur}px">
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
  <Row label="Верхний бар">
    <Toggle
      value={$settings.translucent_topbar}
      onChange={setTopbarTranslucent}
      label="Полупрозрачный верхний бар"
    />
  </Row>
  {#if $settings.translucent_topbar}
    <Row label="Сила блюра" hint="{$settings.topbar_blur}px">
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

<style>
  .bg-preview {
    width: 100%;
    height: 100px;
    border-radius: 8px;
    background: var(--bg-4);
    border: 1px solid var(--border);
    background-size: cover;
    background-position: center;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .bg-actions {
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
    font-size: 12px;
    background: var(--bg-3);
    border: 1px dashed var(--border);
    color: var(--text-3);
    transition: background 0.12s, color 0.12s;
  }
  .action-btn:hover {
    background: var(--bg-4);
    color: var(--text-2);
  }
  .range {
    width: 140px;
    accent-color: var(--accent);
  }

  .hint {
    font-size: 11px;
    color: var(--text-3);
    line-height: 1.4;
  }
  .color-grid {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .color-row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 4px 6px;
    border-radius: 6px;
    transition: background 0.1s;
  }
  .color-row:hover {
    background: var(--bg-3);
  }
  .picker {
    width: 28px;
    height: 28px;
    border: 1px solid var(--border);
    border-radius: 5px;
    padding: 0;
    background: none;
    cursor: pointer;
  }
  .var-name {
    font-size: 11px;
    font-family: "JetBrains Mono", monospace;
    color: var(--text-3);
    width: 88px;
    flex-shrink: 0;
  }
  .var-label {
    flex: 1;
    font-size: 12px;
    color: var(--text-2);
  }
  .row-reset {
    color: var(--text-3);
    padding: 4px;
    border-radius: 4px;
    display: flex;
  }
  .row-reset:hover {
    color: var(--text);
    background: var(--bg-4);
  }
  .reset-btn {
    align-self: flex-start;
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
  .reset-btn:hover {
    background: var(--bg-4);
    color: var(--text-2);
  }
</style>
