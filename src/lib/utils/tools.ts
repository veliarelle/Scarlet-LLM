import { api } from "$lib/api/invoke";
import type { ToolCall } from "$lib/types/chat";
import type { AgentDefinition, Settings } from "$lib/types/settings";

export interface CustomToolDefinition {
  type: "function";
  function: {
    name: string;
    description?: string;
    parameters?: unknown;
  };
  executor?: {
    type: "http";
    method?: string;
    url: string;
    headers?: Record<string, string>;
    body?: unknown;
    timeout_ms?: number;
  };
}

export interface ToolRunResult {
  call: ToolCall;
  content: string;
}

function isToolDefinition(value: unknown): value is CustomToolDefinition {
  if (!value || typeof value !== "object") return false;
  const rec = value as Record<string, unknown>;
  const fn = rec.function;
  return (
    rec.type === "function" &&
    !!fn &&
    typeof fn === "object" &&
    typeof (fn as Record<string, unknown>).name === "string"
  );
}

export function parseToolDefinitions(settings: Settings): CustomToolDefinition[] {
  if (!settings.tools) return [];
  try {
    const parsed = JSON.parse(settings.tool_definitions || "[]");
    if (!Array.isArray(parsed)) return [];
    return parsed.filter(isToolDefinition);
  } catch {
    return [];
  }
}

export function providerToolSchemas(settings: Settings): unknown[] {
  return parseToolDefinitions(settings).map(({ executor: _executor, ...schema }) => schema);
}

export function providerToolsForAgent(settings: Settings, agent: AgentDefinition | null): unknown[] {
  const allowed = new Set((agent?.tool_names ?? []).map((name) => name.trim()).filter(Boolean));
  const all = parseToolDefinitions(settings);
  const selected = allowed.size > 0 ? all.filter((tool) => allowed.has(tool.function.name)) : all;
  return selected.map(({ executor: _executor, ...schema }) => schema);
}

export function toolByName(settings: Settings): Map<string, CustomToolDefinition> {
  const map = new Map<string, CustomToolDefinition>();
  for (const tool of parseToolDefinitions(settings)) {
    map.set(tool.function.name, tool);
  }
  return map;
}

export function parseToolArguments(raw: string): unknown {
  if (!raw.trim()) return {};
  try {
    return JSON.parse(raw);
  } catch {
    return { raw };
  }
}

export async function executeToolCalls(
  calls: ToolCall[],
  settings: Settings,
  agent?: AgentDefinition | null
): Promise<ToolRunResult[]> {
  const defs = toolByName(settings);
  const allowed = new Set((agent?.tool_names ?? []).map((name) => name.trim()).filter(Boolean));
  const out: ToolRunResult[] = [];

  for (const call of calls) {
    if (allowed.size > 0 && !allowed.has(call.name)) {
      out.push({
        call,
        content: `Tool "${call.name}" is not allowed for agent "${agent?.name ?? "unknown"}".`,
      });
      continue;
    }
    const definition = defs.get(call.name);
    if (!definition?.executor) {
      out.push({
        call,
        content: `Tool "${call.name}" is declared, but Scarlet has no local executor configured for it.`,
      });
      continue;
    }

    try {
      const result = await api.executeTool({
        definition,
        arguments: parseToolArguments(call.arguments),
      });
      out.push({ call, content: result.content });
    } catch (e) {
      const message = e instanceof Error ? e.message : String(e);
      out.push({ call, content: `Tool "${call.name}" failed: ${message}` });
    }
  }

  return out;
}
