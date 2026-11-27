import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { PromoteButton } from "./PromoteButton";

vi.mock("@/lib/library", () => ({
	library: { promote: vi.fn(), message: () => "Error" },
}));
describe("promotion", () => {
	it("requires confirmation before promoting literal secrets", () => {
		render(
			<PromoteButton
				document={{
					entry: {
						agent: "codex",
						category: "mcps",
						label: "Config",
						path: "/config",
						format: "toml",
						exists: true,
					},
					content: "token",
					sha256: "hash",
					secretSpans: [{ startByte: 0, endByte: 5, kind: "token" }],
				}}
			/>,
		);
		fireEvent.click(screen.getByRole("button", { name: "Promote to library" }));
		expect(
			screen.getByRole("button", { name: "Confirm promotion" }),
		).toBeDisabled();
		fireEvent.click(screen.getByRole("checkbox"));
		expect(
			screen.getByRole("button", { name: "Confirm promotion" }),
		).toBeEnabled();
	});
});
