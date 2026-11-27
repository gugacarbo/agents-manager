import { commands } from "../bindings/tauri";
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

function formatIpcError(err: unknown): Error {
	if (err instanceof Error) {
		return err;
	}
	if (typeof err === "string") {
		return new Error(err);
	}
	if (typeof err === "object" && err !== null) {
		const coreErr = err as Record<
			string,
			{ message?: string; path?: string; target?: string }
		>;
		if ("Io" in coreErr && coreErr.Io) {
			return new Error(`I/O error: ${coreErr.Io.message}`);
		}
		if ("Serialization" in coreErr && coreErr.Serialization) {
			return new Error(`Serialization error: ${coreErr.Serialization.message}`);
		}
		if ("InvalidPath" in coreErr && coreErr.InvalidPath) {
			return new Error(`Invalid path: ${coreErr.InvalidPath.path}`);
		}
		if ("NotFound" in coreErr && coreErr.NotFound) {
			return new Error(`Not found: ${coreErr.NotFound.target}`);
		}
	}
	return new Error("Unknown IPC error");
}

export async function listAgents(): Promise<AgentSummary[]> {
	if (!isTauriRuntime()) {
		return agentSummarySchema.array().parse(FALLBACK_AGENTS);
	}

	try {
		const raw = await commands.listAgents();
		return agentSummarySchema.array().parse(raw);
	} catch (err) {
		throw formatIpcError(err);
	}
}
