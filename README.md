# Scarlet LLM

Scarlet LLM is a Tauri 2 + SvelteKit desktop and Android frontend for LLM
chat, image generation, proxy profiles, presets, attachments, themes, and local
app settings.

The app is aimed at OpenAI-compatible proxies and native OpenAI, Anthropic, and
Google/Gemini-style providers. It is currently beta software.

## Features

- Chat with OpenAI, Anthropic, Google, and OpenAI-compatible APIs;
- Local oai-reserve proxy manager, together with API keys support;
- Image generation mode for supported Google and OpenAI image models.
- File and image attachments in chats and image-generation requests;
- Prompt and parameter managers. Scarlet LLM does not send anything by default;
- Chat history, message variations, forks, and regeneration;
- Agents, custom tools usage.

## Plans

- Chat groups (claude-like projects);
- Idk for now. 

## Requirements

Install Node.js, npm, Rust, and the platform dependencies required by Tauri.

On Arch Linux:

```bash
sudo pacman -S --needed \
  webkit2gtk-4.1 \
  base-devel \
  curl \
  wget \
  file \
  openssl \
  appmenu-gtk-module \
  libappindicator-gtk3 \
  librsvg \
  xdotool
```

Install Java/Android tooling only if you need Android builds.

## Installation

For Android: Install the apk from releases.
For Linux: Install the appimage/.deb/bin from releases, depending on your Linux distributive.

To compile the app yourself, follow the Development section.

## Development

Install JavaScript dependencies:

```bash
npm install
```

Run the web frontend in development mode:

```bash
npm run dev
```

Run the Tauri desktop app in development mode:

```bash
npm run tauri dev
```

Check the frontend:

```bash
npm run check
```

Check the Rust backend:

```bash
cd src-tauri
cargo check
```

Build only the static frontend:

```bash
npm run build
```

This only writes the web assets to `build/`. It does not create a desktop or
Android application.

## Build: Linux Desktop

From the project root:

```bash
npm install
npm run tauri build
```

Build output is generated under:

```text
src-tauri/target/release/bundle/
```

Depending on the host and Tauri bundle configuration, this can include AppImage,
Debian, and RPM packages.

## Build: Windows

The most reliable way to build Windows packages is on Windows itself:

```bash
npm install
npm run tauri build
```

Tauri can create Windows `.msi` installers only on Windows. If building Windows
from Linux, use the NSIS installer target.

### Cross-build Windows NSIS from Arch Linux

Install tools:

```bash
sudo pacman -S --needed nsis llvm lld clang
rustup target add x86_64-pc-windows-msvc
cargo install --locked cargo-xwin
```

For cross-compilation, set the bundle target in `src-tauri/tauri.conf.json` to
NSIS only:

```json
"bundle": {
  "active": true,
  "targets": ["nsis"]
}
```

Then build:

```bash
npm install
npm run tauri build -- --runner cargo-xwin --target x86_64-pc-windows-msvc
```

Expected output:

```text
src-tauri/target/x86_64-pc-windows-msvc/release/bundle/nsis/
```

## Build: Android

Android builds require the Android SDK, NDK, Java, and a configured Tauri
Android target. This repository already contains the generated Android project
under `src-tauri/gen/android/`.

Debug build:

```bash
npm install
npm run tauri android build -- --debug
```

Release build:

```bash
npm install
npm run tauri android build
```

Typical APK output:

```text
src-tauri/gen/android/app/build/outputs/apk/universal/debug/app-universal-debug.apk
src-tauri/gen/android/app/build/outputs/apk/universal/release/app-universal-release.apk
```

Release signing uses:

```text
src-tauri/gen/android/keystore.properties
```

That file and the keystore are intentionally ignored by git because they contain
private signing data.

Install an APK on a USB-connected Android device:

```bash
~/Android/Sdk/platform-tools/adb devices
~/Android/Sdk/platform-tools/adb install -r \
  "src-tauri/gen/android/app/build/outputs/apk/universal/release/app-universal-release.apk"
```

If Android rejects an update because the old app was signed with a different
key:

```bash
~/Android/Sdk/platform-tools/adb uninstall dev.scarlet.llm
~/Android/Sdk/platform-tools/adb install \
  "src-tauri/gen/android/app/build/outputs/apk/universal/release/app-universal-release.apk"
```
