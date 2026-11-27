import {
	cleanup,
	fireEvent,
	render,
	screen,
	waitFor,
} from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { LibrarySync } from "./LibrarySync";

const { state } = vi.hoisted(() => ({ state: { preview: vi.fn() } }));
vi.mock("@/lib/library", () => ({
	library: { preview: state.preview, message: String },
}));
vi.mock("@/lib/projects", () => ({ projects: { list: async () => [] } }));
vi.mock("@/components/CodeEditor", () => ({
	DiffViewer: ({
		original,
		modified,
	}: {
		original: string;
		modified: string;
	}) => (
		<div>
			{original} → {modified}
		</div>
	),
}));
afterEach(cleanup);

describe("sync review", () => {
	it("shows drift, requires preview, and disables apply when nothing changes", async () => {
		state.preview.mockResolvedValue([
			{
				targetPath: "/config",
				agent: "codex",
				scope: "global",
				strategy: "merge",
				currentContent: "same",
				proposedContent: "same",
				currentSecretSpans: [],
				proposedSecretSpans: [],
				expectedSha256: "hash",
				changed: false,
				warnings: [],
			},
		]);
		render(
			<LibrarySync
				item={{
					id: "1",
					name: "Example",
					category: "mcps",
					sourcePath: "/library",
					hasLiteralSecrets: false,
					targets: [
						{
							agent: "codex",
							scope: "global",
							path: "/config",
							status: "drifted",
						},
					],
				}}
				onEdit={() => {}}
				onRefresh={async () => {}}
				onClose={() => {}}
			/>,
		);
		expect(screen.getByText(/Drifted/)).toBeInTheDocument();
		expect(
			screen.queryByRole("button", { name: "Apply sync" }),
		).not.toBeInTheDocument();
		fireEvent.click(screen.getByRole("checkbox", { name: /Codex · Global/ }));
		fireEvent.click(screen.getByRole("button", { name: "Preview sync" }));
		expect(
			await screen.findByRole("button", { name: "Apply sync" }),
		).toBeDisabled();
	});
	it("sends copy only after the user explicitly selects the fallback", async () => {
		state.preview.mockResolvedValue([
			{
				targetPath: "/rule",
				agent: "codex",
				scope: "global",
				strategy: "copy",
				currentContent: null,
				proposedContent: "text",
				currentSecretSpans: [],
				proposedSecretSpans: [],
				expectedSha256: null,
				changed: true,
				warnings: [],
			},
		]);
		render(
			<LibrarySync
				item={{
					id: "rule",
					name: "Rule",
					category: "rules",
					sourcePath: "/lib",
					hasLiteralSecrets: false,
					targets: [],
				}}
				onEdit={() => {}}
				onRefresh={async () => {}}
				onClose={() => {}}
			/>,
		);
		fireEvent.click(screen.getByRole("checkbox", { name: "Codex · Global" }));
		fireEvent.click(
			screen.getByRole("checkbox", { name: /Use copy for Codex/ }),
		);
		fireEvent.click(screen.getByRole("button", { name: "Preview sync" }));
		await waitFor(() =>
			expect(state.preview).toHaveBeenCalledWith("rule", [
				expect.objectContaining({ agent: "codex", copy: true }),
			]),
		);
	});
});
