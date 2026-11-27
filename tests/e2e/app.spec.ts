import { expect, test } from "@playwright/test";

test("displays Agents Manager heading and the three agent cards", async ({
	page,
}) => {
	await page.goto("/");

	await expect(
		page.getByRole("heading", { name: "Agents Manager" }),
	).toBeVisible();

	await expect(
		page.getByRole("heading", { name: "Claude Code" }),
	).toBeVisible();
	await expect(page.getByRole("heading", { name: "Codex" })).toBeVisible();
	await expect(
		page.getByRole("heading", { name: "Antigravity" }),
	).toBeVisible();

	await expect(page.getByText("Not detected")).toHaveCount(3);
});
