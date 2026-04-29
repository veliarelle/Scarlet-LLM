export type Role = "system" | "user" | "assistant" | "tool";

export interface TextPart { type: "text"; text: string }
export interface ImagePart { type: "image_url"; image_url: { url: string } }
export interface FilePart  { type: "file"; source: { type: "base64"; media_type: string; data: string }; name?: string }
export type ContentPart = TextPart | ImagePart | FilePart;

export interface Attachment {
  id: string;
  name: string;
  mimeType: string;
  data: string; // base64, no "data:" prefix
  text?: string | null;
}

// Сетевой формат сообщения (что отправляем в LLM):
export interface ChatMessage {
  role: Role;
  content: string | ContentPart[];
  name?: string;
  tool_call_id?: string;
  tool_calls?: ToolCall[];
}

export interface ToolCall {
  id: string;
  name: string;
  arguments: string;
}

export interface VariationMeta {
  model?: string | null;
  created_at: string; // ISO
  image_url?: string | null;
}

export interface ChatBookmark {
  id: string;
  message_id: string;
  leaf_id?: string | null;
  label: string;
  created_at: string;
}

// Сообщение в чате (с метаданными):
export interface Message {
  id: string;
  role: Role;
  content: string;
  created_at: string; // ISO
  parent_id?: string | null;
  child_ids?: string[];
  active_child_id?: string | null;
  model?: string | null;
  variations?: string[];
  variation_index?: number;
  variation_meta?: VariationMeta[];
  image_url?: string | null;
  attachments?: Attachment[];
  bookmarked?: boolean;
}

export interface Chat {
  id: string;
  title: string;
  pinned: boolean;
  created_at: string;
  updated_at: string;
  model?: string | null;
  proxy_id?: string | null;
  summary?: ChatSummary | null;
  active_leaf_id?: string | null;
  bookmarks?: ChatBookmark[];
  messages: Message[];
}

export interface ChatSummary {
  id: string;
  content: string;
  prompt: string;
  after_message_id: string;
  model?: string | null;
  created_at: string;
  updated_at: string;
}

export interface ChatMeta {
  id: string;
  title: string;
  pinned: boolean;
  created_at: string;
  updated_at: string;
}

export interface NewChatInput {
  title?: string | null;
  model?: string | null;
  proxy_id?: string | null;
}

export interface Model {
  id: string;
  name?: string | null;
}

export interface TokenUsage {
  prompt_tokens: number;
  completion_tokens: number;
  total_tokens: number;
}

export interface CompletionResponse {
  content: string;
  usage?: TokenUsage | null;
  image_url?: string | null;
  tool_calls?: ToolCall[];
}

export interface ToolExecutionInput {
  definition: unknown;
  arguments: unknown;
}

export interface ToolExecutionResponse {
  content: string;
}

export interface SendCompletionInput {
  proxy_id: string;
  model: string;
  messages: ChatMessage[];
  params?: Record<string, unknown>;
  tools?: unknown[];
  web_search?: boolean;
  prompt_caching?: boolean;
}

export interface ImageGenInput {
  proxy_id: string;
  model: string;
  prompt: string;
  image_id?: string;
  attachments?: Attachment[];
  params?: Record<string, unknown>;
}

export interface ImageGenResponse {
  url: string;
}

export type StreamEvent =
  | { type: "chunk"; content: string }
  | { type: "done"; usage?: TokenUsage | null }
  | { type: "error"; message: string };
