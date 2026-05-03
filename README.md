# Scarlet LLM

Scarlet LLM is a Tauri 2 + SvelteKit desktop and Android frontend for LLM
chat, image generation, proxy profiles, presets, attachments, themes, and local
app settings.

The app is aimed at simplicity and minimal design - similar to claude/chatgpt official apps and websites.

## Features

- Chat with OpenAI, Anthropic, Google, and OpenAI-compatible APIs;
- Local oai-reserve proxy manager, together with API keys support;
- Image generation mode for supported Google and OpenAI image models;
- File and image attachments in chats and image-generation requests;
- Prompt and parameter managers. Scarlet LLM does not send anything by default;
- Chat history, message variations, forks, and regeneration;
- Agents, custom tools usage.

## Plans

- Chat groups (claude-like projects);
- Complete markdown support (.md);
- Terminal mode;
- Chat tabs and split view;
- Regex post-processing. 

## Tech Stack

- **Frontend:** Svelte 5, SvelteKit, TypeScript, lucide-svelte;
- **Backend:** Tauri 2, Rust commands, local JSON app storage;
- **Build:** Vite, Tauri CLI, Cargo.

## Prerequisites

- Node.js and npm;
- Rust via rustup;
- Platform dependencies required by Tauri 2;
- Android builds only: Android Studio or Android SDK/NDK tooling, `JAVA_HOME`, `ANDROID_HOME`, `NDK_HOME`, and Android Rust targets;
- Windows builds only: Windows build tools and WebView2 are required when building on Windows.

## Installation

For Android: Install the apk from releases.
For Linux: Install the appimage/.deb/bin from releases, depending on your Linux distributive.
For Windows: The releases for Windows can be delayed, but if available - then download from releases.

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

Run the Scarlet-LLM in development mode:

```bash
npm run tauri dev
```

Build the app from the project root:

```bash
npm run tauri build
```

## License

Licensed under the **MIT License.**
