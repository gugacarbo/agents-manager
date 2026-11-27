import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { ProjectsPage } from "./ProjectsPage";

vi.mock("@/lib/projects", () => ({
	projects: {
		list: async () => [
			{
				id: "1",
				name: "Missing",
				rootPath: "/missing",
				addedAt: "today",
				available: false,
			},
		],
		register: vi.fn(),
		remove: vi.fn(),
		message: () => "Error",
	},
}));
describe("projects", () => {
	it("shows unavailable projects and offers removal", async () => {
		render(<ProjectsPage />);
		expect(await screen.findByText("Unavailable")).toBeInTheDocument();
		expect(
			screen.getByRole("button", { name: /Remove from list/ }),
		).toBeInTheDocument();
	});
});
