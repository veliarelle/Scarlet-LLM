export type ProxyKind =
  | "openai_compat"
  | "anthropic_native"
  | "google_native"
  | "openai_responses";

export interface Proxy {
  id: string;
  name: string;
  base_url: string;
  key: string;
  kind: ProxyKind;
  created_at: string;
}

export interface ProxyInput {
  name: string;
  base_url: string;
  key: string;
  kind: ProxyKind;
}

export const PROXY_KIND_LABELS: Record<ProxyKind, string> = {
  openai_compat: "OpenAI-compatible",
  anthropic_native: "Anthropic native",
  google_native: "Google native",
  openai_responses: "OpenAI Responses API",
};
