import type { Role } from "./chat";

export interface ParamEntry {
  id: string;
  key: string;
  value: string;
  enabled: boolean;
}

export interface ReasoningConfig {
  enabled: boolean;
  effort: "low" | "medium" | "high";
  send_effort: boolean;
}

export interface Prompt {
  id: string;
  name: string;
  role: Role;
  content: string;
  enabled: boolean;
}

export interface PromptUtilities {
  summarize_prompt_id?: string | null;
  summarize_default_prompt: string;
  auto_summarize: boolean;
}

export interface PresetUtilities {
  summarize_prompt_id?: string | null;
}

export interface AgentDefinition {
  id: string;
  name: string;
  description: string;
  prompt: string;
  enabled: boolean;
  model?: string | null;
  proxy_id?: string | null;
  tool_names: string[];
}

export interface AgentPreset {
  id: string;
  name: string;
  agents: AgentDefinition[];
  created_at: string;
  updated_at: string;
}

export interface AgentPresetMeta {
  id: string;
  name: string;
  created_at: string;
  updated_at: string;
}

export type Theme = "dark" | "light" | "custom";
export type Language = "ru" | "en";

export const DEFAULT_SUMMARIZE_PROMPT =
  "Summarize the conversation so it can replace the earlier chat history. Keep the user's goals, decisions, constraints, important facts, file/image context, unresolved tasks, and the latest state. Be concise but specific. Do not answer the user; only produce the summary.";

export const COLOR_VARS = [
  "bg",
  "bg-2",
  "bg-3",
  "bg-4",
  "border",
  "accent",
  "accent-h",
  "accent-d",
  "text",
  "text-2",
  "text-3",
  "danger",
] as const;
export type ColorVar = (typeof COLOR_VARS)[number];

export interface Settings {
  language: Language;
  active_proxy_id: string | null;
  active_preset_id: string | null;
  active_agent_preset_id: string | null;
  active_model: string | null;
  active_chat_id: string | null;
  user_name: string;
  assistant_name: string;
  streaming: boolean;
  context_window: number;
  max_tokens: number;
  max_message_size: number;
  show_token_counts: boolean;
  prompt_caching: boolean;
  params: ParamEntry[];
  reasoning: ReasoningConfig;
  prompts: Prompt[];
  utilities: PromptUtilities;
  web_search: boolean;
  agents: boolean;
  agent_definitions: AgentDefinition[];
  tools: boolean;
  tool_definitions: string; // JSON array of tool objects
  theme: Theme;
  custom_colors: Record<string, string>;
  ui_scale: number;
  translucent_sidebar: boolean;
  sidebar_blur: number;
  translucent_topbar: boolean;
  topbar_blur: number;
}

export const DEFAULT_SETTINGS: Settings = {
  language: "ru",
  active_proxy_id: null,
  active_preset_id: null,
  active_agent_preset_id: null,
  active_model: null,
  active_chat_id: null,
  user_name: "User",
  assistant_name: "Scarlet",
  streaming: true,
  context_window: 8192,
  max_tokens: 2048,
  max_message_size: 0,
  show_token_counts: false,
  prompt_caching: false,
  params: [
    { id: "tp", key: "temperature", value: "0.7", enabled: true },
    { id: "pp", key: "top_p", value: "0.9", enabled: true },
  ],
  reasoning: { enabled: false, effort: "medium", send_effort: true },
  prompts: [],
  utilities: {
    summarize_prompt_id: null,
    summarize_default_prompt: DEFAULT_SUMMARIZE_PROMPT,
    auto_summarize: false,
  },
  web_search: false,
  agents: false,
  agent_definitions: [],
  tools: false,
  theme: "dark",
  custom_colors: {},
  ui_scale: 1.0,
  translucent_sidebar: false,
  sidebar_blur: 8,
  translucent_topbar: false,
  topbar_blur: 8,
  tool_definitions: "[]",
};
