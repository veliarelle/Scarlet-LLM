import type { ChatMessage } from "$lib/types/chat";
import type { AgentDefinition, Settings } from "$lib/types/settings";

function escapeRegex(value: string): string {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

export function resolveMentionedAgent(text: string, settings: Settings): AgentDefinition | null {
  if (!settings.agents) return null;
  const agents = settings.agent_definitions ?? [];
  return (
    agents.find((agent) => {
      if (!agent.enabled || !agent.name.trim()) return false;
      const pattern = new RegExp(`(^|\\s)@${escapeRegex(agent.name.trim())}(?=$|\\s|[.,:;!?])`, "i");
      return pattern.test(text);
    }) ?? null
  );
}

export function resolveAgentForRequest(userText: string, settings: Settings): AgentDefinition | null {
  const fromUser = resolveMentionedAgent(userText, settings);
  if (fromUser) return fromUser;

  const promptText = (settings.prompts ?? [])
    .filter((prompt) => prompt.enabled && prompt.content.trim())
    .map((prompt) => prompt.content)
    .join("\n\n");
  return resolveMentionedAgent(promptText, settings);
}

export function applyAgentSettings(settings: Settings, agent: AgentDefinition | null): Settings {
  if (!agent) return settings;
  return {
    ...settings,
    active_model: agent.model?.trim() || settings.active_model,
    active_proxy_id: agent.proxy_id?.trim() || settings.active_proxy_id,
  };
}

export function agentSystemMessage(agent: AgentDefinition | null): ChatMessage | null {
  if (!agent?.prompt.trim()) return null;
  const description = agent.description.trim()
    ? `\nAgent description: ${agent.description.trim()}`
    : "";
  return {
    role: "system",
    content: `You are the named Scarlet agent "${agent.name.trim()}".${description}\n\n${agent.prompt.trim()}`,
  };
}

export function applyAgentPrompt(
  messages: ChatMessage[],
  agent: AgentDefinition | null
): ChatMessage[] {
  const system = agentSystemMessage(agent);
  return system ? [system, ...messages] : messages;
}
