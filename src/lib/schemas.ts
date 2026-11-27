import { z } from "zod";
import type { AgentSummary_Serialize } from "../bindings/tauri";

export const agentKindSchema = z.enum(["claude-code", "codex", "antigravity"]);

export const agentSummarySchema = z.object({
	kind: agentKindSchema,
	name: z.string().min(1),
	detected: z.boolean(),
	configPath: z.string().nullable(),
}) satisfies z.ZodType<AgentSummary_Serialize>;

export type AgentSummary = z.infer<typeof agentSummarySchema>;
