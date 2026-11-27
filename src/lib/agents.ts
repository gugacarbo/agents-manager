import { commands } from "../bindings/tauri";
import { formatCoreError } from "./ipc-errors";
import { type AgentSummary, agentSummarySchema } from "./schemas";

const FALLBACK_AGENTS: AgentSummary[] = [
	{
		kind: "claude-code",
		name: "Claude Code",
		detected: false,
		configPath: null,
	},
	{
		kind: "codex",
		name: "Codex",
		detected: false,
		configPath: null,
	},
	{
		kind: "antigravity",
		name: "Antigravity",
		detected: false,
		configPath: null,
	},
];

export function isTauriRuntime(): boolean {
	return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

export async function listAgents(): Promise<AgentSummary[]> {
	if (!isTauriRuntime()) {
		return agentSummarySchema.array().parse(FALLBACK_AGENTS);
	}

	try {
		const raw = await commands.listAgents();
		return agentSummarySchema.array().parse(raw);
	} catch (err) {
		throw formatCoreError(err);
	}
}
