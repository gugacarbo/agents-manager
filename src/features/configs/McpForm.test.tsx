import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { McpForm } from "./McpForm";

describe("MCP form", () => {
	it("masks env values and explains environment references", () => {
		render(
			<McpForm
				servers={[
					{
						name: "demo",
						transport: "stdio",
						command: "node",
						env: [
							{
								key: "TOKEN",
								value: "secret",
								isSecret: true,
								isEnvRef: false,
							},
						],
						enabled: true,
					},
				]}
				onChange={vi.fn()}
			/>,
		);
		const value = screen.getByLabelText("Environment value 1");
		expect(value).toHaveAttribute("type", "password");
		expect(
			screen.getByText(/environment variable reference/),
		).toBeInTheDocument();
		fireEvent.click(screen.getByRole("button", { name: "Reveal" }));
		expect(value).toHaveAttribute("type", "text");
	});
});
