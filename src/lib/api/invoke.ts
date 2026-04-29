import { invoke, Channel } from "@tauri-apps/api/core";
import type { Proxy, ProxyInput } from "$lib/types/proxy";
import type { AgentDefinition, AgentPreset, AgentPresetMeta, PresetUtilities, Prompt, Settings } from "$lib/types/settings";
import type { Preset, PresetMeta } from "$lib/types/preset";
import type {
  Chat,
  ChatMeta,
  CompletionResponse,
  ImageGenInput,
  ImageGenResponse,
  Attachment,
  Model,
  NewChatInput,
  SendCompletionInput,
  StreamEvent,
  ToolExecutionInput,
  ToolExecutionResponse,
} from "$lib/types/chat";

export const api = {
  ping: () => invoke<string>("ping"),

  getSettings: () => invoke<Settings>("get_settings"),
  saveSettings: (settings: Settings) => invoke<Settings>("save_settings", { settings }),

  listProxies: () => invoke<Proxy[]>("list_proxies"),
  createProxy: (input: ProxyInput) => invoke<Proxy>("create_proxy", { input }),
  updateProxy: (id: string, input: ProxyInput) =>
    invoke<Proxy>("update_proxy", { id, input }),
  deleteProxy: (id: string) => invoke<void>("delete_proxy", { id }),

  listModels: (proxyId: string) => invoke<Model[]>("list_models", { proxyId }),
  sendCompletion: (input: SendCompletionInput) =>
    invoke<CompletionResponse>("send_completion", { input }),
  sendCompletionCancellable: (input: SendCompletionInput, requestId: string) =>
    invoke<CompletionResponse>("send_completion_cancellable", { input, requestId }),

  streamCompletion(
    input: SendCompletionInput,
    streamId: string,
    onEvent: (e: StreamEvent) => void
  ): Promise<void> {
    const channel = new Channel<StreamEvent>();
    channel.onmessage = onEvent;
    return invoke<void>("stream_completion", { input, streamId, onEvent: channel });
  },
  cancelStream: (streamId: string) => invoke<void>("cancel_stream", { streamId }),
  cancelGeneration: (id: string) => invoke<void>("cancel_stream", { streamId: id }),
  executeTool: (input: ToolExecutionInput) =>
    invoke<ToolExecutionResponse>("execute_tool", { input }),

  generateImage: (input: ImageGenInput) =>
    invoke<ImageGenResponse>("generate_image", { input }),
  saveImage: (dataUrl: string, defaultName: string, title?: string) =>
    invoke<boolean>("save_image", { dataUrl, defaultName, title }),
  prepareAttachments: (files: { name: string; mimeType: string; data: string }[]) =>
    invoke<Attachment[]>("prepare_attachments", { files }),
  readDroppedFiles: (paths: string[]) =>
    invoke<Attachment[]>("read_dropped_files", { paths }),

  listChats: () => invoke<ChatMeta[]>("list_chats"),
  loadChat: (id: string) => invoke<Chat>("load_chat", { id }),
  saveChat: (chat: Chat) => invoke<Chat>("save_chat", { chat }),
  createChat: (input: NewChatInput) => invoke<Chat>("create_chat", { input }),
  deleteChat: (id: string) => invoke<void>("delete_chat", { id }),
  renameChat: (id: string, title: string) =>
    invoke<ChatMeta>("rename_chat", { id, title }),
  pinChat: (id: string, pinned: boolean) =>
    invoke<ChatMeta>("pin_chat", { id, pinned }),
  forkChat: (id: string, untilMessageId: string) =>
    invoke<Chat>("fork_chat", { id, untilMessageId }),

  listPresets: () => invoke<PresetMeta[]>("list_presets"),
  loadPreset: (id: string) => invoke<Preset>("load_preset", { id }),
  savePreset: (preset: Preset) => invoke<Preset>("save_preset", { preset }),
  createPreset: (name: string, prompts: Prompt[], utilities?: PresetUtilities) =>
    invoke<Preset>("create_preset", { name, prompts, utilities }),
  deletePreset: (id: string) => invoke<void>("delete_preset", { id }),

  listAgentPresets: () => invoke<AgentPresetMeta[]>("list_agent_presets"),
  loadAgentPreset: (id: string) => invoke<AgentPreset>("load_agent_preset", { id }),
  saveAgentPreset: (preset: AgentPreset) => invoke<AgentPreset>("save_agent_preset", { preset }),
  createAgentPreset: (name: string, agents: AgentDefinition[]) =>
    invoke<AgentPreset>("create_agent_preset", { name, agents }),
  deleteAgentPreset: (id: string) => invoke<void>("delete_agent_preset", { id }),

  exportPreset: (presetId: string) => invoke<boolean>("export_preset", { presetId }),
  importPreset: () => invoke<Preset | null>("import_preset"),
  exportAgentPreset: (presetId: string) => invoke<boolean>("export_agent_preset", { presetId }),
  importAgentPreset: () => invoke<AgentPreset | null>("import_agent_preset"),
  exportProfile: () => invoke<boolean>("export_profile"),
  importProfile: () => invoke<number>("import_profile"),
};
