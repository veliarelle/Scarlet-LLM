# Scarlet LLM

Scarlet LLM is a Tauri + SvelteKit desktop frontend for working with LLM
proxies, chats, presets, tools, themes, and local app settings.

## Development

```sh
npm install
npm run dev
npm run check
```

## UI scale layout bug

The UI scale setting is stored as `settings.ui_scale` and exposed to CSS as
`--ui-scale`. The broken version applied it with CSS `zoom` directly on the
main `.root` element while that same element had `height: 100vh`.

That combination is unsafe for this app: `100vh` is measured before the visual
zoom is applied, so a scale above `1` makes the already viewport-sized layout
render larger than the window. The bottom and right edges are clipped, fixed
panels no longer line up with the scaled content, and the whole interface looks
shifted. A scale below `1` creates the opposite mismatch: the rendered UI becomes
smaller than the viewport, leaving dead space and misaligned overlays.

The fix is to avoid `zoom` for the root layout. The app now uses a fixed
`.scale-shell` wrapper with inverse logical dimensions:

```css
width: calc(100vw / var(--ui-scale, 1));
height: calc(100vh / var(--ui-scale, 1));
transform: scale(var(--ui-scale, 1));
transform-origin: top left;
```

After the transform is applied, the shell visually fits the viewport again, and
the sidebar, top bar, chat area, and settings drawer stay in the same coordinate
space.
