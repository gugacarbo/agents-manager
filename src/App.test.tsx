import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { App } from "./App";
import { agentKindSchema, agentSummarySchema } from "./lib/schemas";

function renderApp() {
	const queryClient = new QueryClient({
		defaultOptions: {
			queries: {
				retry: false,
			},
		},
	});

	return render(
		<QueryClientProvider client={queryClient}>
			<App />
		</QueryClientProvider>,
	);
}

afterEach(() => {
	cleanup();
	clearMocks();
	delete (window as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;
});

describe("agentSummarySchema", () => {
	it("accepts valid agent summaries", () => {
		const validDetected = agentSummarySchema.parse({
			kind: "claude-code",
			name: "Claude Code",
			detected: true,
			configPath: "/home/user/.claude",
		});

		const validFallback = agentSummarySchema.parse({
			kind: "codex",
			name: "Codex",
			detected: false,
			configPath: null,
		});

		expect(validDetected.detected).toBe(true);
		expect(validFallback.configPath).toBeNull();
		expect(agentKindSchema.options).toEqual([
			"claude-code",
			"codex",
			"antigravity",
		]);
	});

	it("rejects invalid agent kinds, empty names, and wrong types", () => {
		expect(() =>
			agentSummarySchema.parse({
				kind: "unknown-agent",
				name: "Unknown",
				detected: false,
				configPath: null,
			}),
		).toThrow();

		expect(() =>
			agentSummarySchema.parse({
				kind: "antigravity",
				name: "",
				detected: true,
				configPath: null,
			}),
		).toThrow();

		expect(() =>
			agentSummarySchema.parse({
				kind: "codex",
				name: "Codex",
				detected: "yes",
				configPath: null,
			}),
		).toThrow();
	});
});

describe("App", () => {
	it("renders the Agents Manager heading and fallback cards outside Tauri", async () => {
		renderApp();

		expect(
			screen.getByRole("heading", { name: "Agents Manager" }),
		).toBeInTheDocument();

		expect(
			await screen.findByRole("heading", { name: "Claude Code" }),
		).toBeInTheDocument();
		expect(screen.getByRole("heading", { name: "Codex" })).toBeInTheDocument();
		expect(
			screen.getByRole("heading", { name: "Antigravity" }),
		).toBeInTheDocument();

		const statuses = screen.getAllByRole("status");
		expect(statuses).toHaveLength(3);
		for (const status of statuses) {
			expect(status).toHaveTextContent("Not detected");
		}
	});

	it("renders detected agents and config paths when running inside Tauri IPC", async () => {
		mockIPC((cmd) => {
			if (cmd === "list_agents") {
				return [
					{
						kind: "claude-code",
						name: "Claude Code",
						detected: true,
						configPath: "/home/dev/.claude",
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
						detected: true,
						configPath: "/home/dev/.gemini/antigravity",
					},
				];
			}
			return null;
		});

		renderApp();

		expect(await screen.findByText("/home/dev/.claude")).toBeInTheDocument();
		expect(
			screen.getByText("/home/dev/.gemini/antigravity"),
		).toBeInTheDocument();

		const statuses = screen.getAllByRole("status");
		expect(statuses).toHaveLength(3);
		expect(statuses[0]).toHaveTextContent("Detected");
		expect(statuses[1]).toHaveTextContent("Not detected");
		expect(statuses[2]).toHaveTextContent("Detected");
	});

	it("renders an alert when Tauri IPC rejects with a CoreError", async () => {
		mockIPC((cmd) => {
			if (cmd === "list_agents") {
				throw { NotFound: { target: "user home directory" } };
			}
			return null;
		});

		renderApp();

		const alert = await screen.findByRole("alert");
		expect(alert).toHaveTextContent(
			"Failed to load agents: Not found: user home directory",
		);
	});
});
