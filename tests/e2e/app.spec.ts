import { expect, test } from "@playwright/test";

test("displays Agents Manager heading, cards, table, and editor route", async ({
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

	await expect(
		page.getByRole("status").filter({ hasText: "Not detected" }),
	).toHaveCount(3);

	await page.getByRole("link", { name: "Editor & Diff" }).click();
	await expect(
		page.getByRole("heading", { name: "Config & Diff Preview" }),
	).toBeVisible();
});
