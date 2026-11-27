import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { SyncPreviewList } from "./SyncPreviewList";

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
describe("sync preview", () => {
	it("masks secrets in both versions without changing other content", () => {
		render(
			<SyncPreviewList
				items={[
					{
						targetPath: "/target",
						agent: "codex",
						scope: "global",
						strategy: "merge",
						currentContent: "token=old-value",
						proposedContent: "token=new-value",
						currentSecretSpans: [{ startByte: 6, endByte: 15, kind: "token" }],
						proposedSecretSpans: [{ startByte: 6, endByte: 15, kind: "token" }],
						expectedSha256: "hash",
						changed: true,
						warnings: [],
					},
				]}
			/>,
		);
		expect(screen.getByText(/token=••••••/)).toBeInTheDocument();
		expect(screen.queryByText(/old-value|new-value/)).not.toBeInTheDocument();
	});
});
