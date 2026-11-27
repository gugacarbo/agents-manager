import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import {
	cleanup,
	fireEvent,
	render,
	screen,
	within,
} from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { App } from "./App";
import { CodeEditor } from "./components/CodeEditor";
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
	window.location.hash = "";
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
	it("lists each supported agent once when no configurations are detected", async () => {
		renderApp();
		const list = await screen.findByRole("list", { name: "Agent inventory" });
		expect(within(list).getAllByRole("listitem")).toHaveLength(3);
		for (const name of ["Claude Code", "Codex", "Antigravity"]) {
			expect(screen.getAllByText(name, { exact: true })).toHaveLength(1);
		}
		expect(within(list).getAllByText("Not detected")).toHaveLength(3);
		expect(
			screen.getByText("0 of 3 configurations detected"),
		).toBeInTheDocument();
	});

	it("clears an unmatched query without changing the detection count", async () => {
		renderApp();
		const search = await screen.findByRole("searchbox", {
			name: "Search by agent or path",
		});
		fireEvent.change(search, { target: { value: "unmatched" } });
		expect(
			screen.getByText("No agents match your filters."),
		).toBeInTheDocument();
		expect(
			screen.getByText("0 of 3 configurations detected"),
		).toBeInTheDocument();
		fireEvent.click(screen.getByRole("button", { name: "Clear filters" }));
		expect(search).toHaveValue("");
		expect(screen.getAllByRole("listitem")).toHaveLength(3);
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

		expect(await screen.findAllByText("/home/dev/.claude")).not.toHaveLength(0);
		expect(
			screen.getAllByText("/home/dev/.gemini/antigravity"),
		).not.toHaveLength(0);

		const list = screen.getByRole("list", { name: "Agent inventory" });
		expect(within(list).getAllByText("Detected")).toHaveLength(2);
		expect(within(list).getAllByText("Not detected")).toHaveLength(1);
		expect(
			screen.getByText("2 of 3 configurations detected"),
		).toBeInTheDocument();
		fireEvent.change(screen.getByRole("searchbox"), {
			target: { value: "/home/dev/.claude" },
		});
		expect(within(list).getAllByRole("listitem")).toHaveLength(1);
		expect(within(list).getByText("Claude Code")).toBeInTheDocument();
	});

	it("retries the first failed detection without presenting it as zero detections", async () => {
		let fail = true;
		mockIPC(() => {
			if (fail) throw { NotFound: { target: "user home directory" } };
			return [
				{ kind: "codex", name: "Codex", detected: false, configPath: null },
			];
		});
		renderApp();
		expect(await screen.findByRole("alert")).toHaveTextContent(
			"Could not check local configuration folders",
		);
		expect(
			screen.queryByText(/configurations detected/),
		).not.toBeInTheDocument();
		fail = false;
		fireEvent.click(screen.getByRole("button", { name: "Try again" }));
		expect(await screen.findByText("Codex")).toBeInTheDocument();
	});

	it("keeps previous results during and after an unsuccessful refresh", async () => {
		let rejectRefresh: (reason: unknown) => void = () => {};
		const refresh = new Promise((_resolve, reject) => {
			rejectRefresh = reject;
		});
		let firstCall = true;
		mockIPC(() => {
			if (!firstCall) return refresh;
			firstCall = false;
			return [
				{
					kind: "codex",
					name: "Codex",
					detected: true,
					configPath: "/home/dev/.codex",
				},
			];
		});
		renderApp();
		expect(await screen.findByText("/home/dev/.codex")).toBeInTheDocument();
		fireEvent.click(screen.getByRole("button", { name: "Refresh" }));
		expect(
			await screen.findByRole("button", { name: "Refreshing…" }),
		).toBeDisabled();
		expect(screen.getByText("/home/dev/.codex")).toBeInTheDocument();
		rejectRefresh("Detection failed");
		expect(await screen.findByRole("alert")).toHaveTextContent(
			"Could not refresh. Showing previous results.",
		);
		expect(screen.getByText("/home/dev/.codex")).toBeInTheDocument();
	});
});

describe("CodeEditor", () => {
	it("mounts CodeMirror editor with initial content", () => {
		render(<CodeEditor value='{"ok": true}' language="json" />);
		expect(screen.getByLabelText("Configuration editor")).toBeInTheDocument();
	});
});
