import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { LibraryPage } from "./LibraryPage";

vi.mock("@/lib/library", () => ({
	library: { list: async () => [], message: () => "Error" },
}));
describe("Library", () => {
	it("invites optional creation when empty", async () => {
		render(<LibraryPage />);
		expect(await screen.findByText(/no library items/i)).toBeInTheDocument();
		expect(
			screen.getByRole("button", { name: /create item/i }),
		).toBeInTheDocument();
	});
});
