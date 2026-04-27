// Применение фонового изображения. Хранится в localStorage (base64 + blur + dim).
// Применяется глобально при старте приложения и при каждом изменении в Appearance.

const KEY_IMG = "sl-bg";
const KEY_BLUR = "sl-bg-blur";
const KEY_DIM = "sl-bg-dim";

export interface BgState {
  hasBg: boolean;
  dataUrl: string;
  blur: number;
  dim: number;
}

export function readBg(): BgState {
  try {
    const dataUrl = localStorage.getItem(KEY_IMG) ?? "";
    return {
      hasBg: !!dataUrl,
      dataUrl,
      blur: Number(localStorage.getItem(KEY_BLUR) ?? "0"),
      dim: Number(localStorage.getItem(KEY_DIM) ?? "0.4"),
    };
  } catch {
    return { hasBg: false, dataUrl: "", blur: 0, dim: 0.4 };
  }
}

export function writeBgImage(dataUrl: string) {
  try {
    localStorage.setItem(KEY_IMG, dataUrl);
  } catch (e) {
    throw new Error(
      "Изображение слишком большое для хранения. Сожми/уменьши и попробуй снова. " +
        String(e)
    );
  }
}

export function clearBgImage() {
  localStorage.removeItem(KEY_IMG);
  localStorage.removeItem(KEY_BLUR);
  localStorage.removeItem(KEY_DIM);
}

export function writeBgBlur(blur: number) {
  localStorage.setItem(KEY_BLUR, String(blur));
}

export function writeBgDim(dim: number) {
  localStorage.setItem(KEY_DIM, String(dim));
}

export function applyBackground(state: BgState = readBg()) {
  if (typeof document === "undefined") return;
  const layer = document.getElementById("bg-layer");
  const dim = document.getElementById("bg-dim");
  if (!layer || !dim) return;

  if (state.hasBg && state.dataUrl) {
    layer.style.backgroundImage = `url(${state.dataUrl})`;
    layer.style.backgroundSize = "cover";
    layer.style.backgroundPosition = "center";
    layer.style.filter = `blur(${state.blur}px)`;
    // Расширяем слой за пределы viewport, чтобы blur не обрезался по краям.
    // Используем inset вместо transform — transform создаёт лишний stacking context,
    // из-за чего часть UI могла "пропадать" при первом применении.
    const overshoot = Math.max(state.blur * 2, 6);
    layer.style.inset = `-${overshoot}px`;
    dim.style.background = `rgba(0,0,0,${state.dim})`;
  } else {
    layer.style.backgroundImage = "none";
    layer.style.filter = "";
    layer.style.inset = "0";
    dim.style.background = "none";
  }
}
