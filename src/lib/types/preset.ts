import type { Prompt } from "./settings";

export interface Preset {
  id: string;
  name: string;
  prompts: Prompt[];
  created_at: string;
  updated_at: string;
}

export interface PresetMeta {
  id: string;
  name: string;
  created_at: string;
  updated_at: string;
}
