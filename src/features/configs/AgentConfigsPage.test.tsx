import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { AgentConfigsPage } from "./AgentConfigsPage";

vi.mock("@/lib/configs", () => ({
	configs: {
		listForScope: async () => [
			{
				agent: "codex",
				category: "settings",
				label: "Config",
				path: "/config.toml",
				format: "toml",
				exists: true,
			},
			{
				agent: "codex",
				category: "rules",
				label: "Rules",
				path: "/AGENTS.md",
				format: "markdown",
				exists: false,
			},
		],
		read: async () => ({
			entry: {
				agent: "codex",
				category: "settings",
				label: "Config",
				path: "/config.toml",
				format: "toml",
				exists: true,
			},
			content: 'a = "ok"',
			sha256: "old",
			secretSpans: [],
		}),
		watch: async () => () => {},
		save: vi.fn(),
		message: () => "Error",
	},
}));

describe("global configuration", () => {
	it("lists discovered and missing files and opens a detected file", async () => {
		render(<AgentConfigsPage agentKind="codex" />);
		expect(await screen.findByText(/Not found/)).toBeInTheDocument();
		fireEvent.click(
			screen.getByRole("button", { name: /Config.*\/config.toml/ }),
		);
		expect(
			await screen.findByRole("textbox", { name: "Configuration editor" }),
		).toBeInTheDocument();
		expect(screen.getByRole("button", { name: "Save" })).toBeDisabled();
	});
});
