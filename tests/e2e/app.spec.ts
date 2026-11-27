import { expect, test } from "@playwright/test";

for (const width of [1200, 900]) {
	test(`keeps a long configuration path readable at ${width}px`, async ({
		page,
	}) => {
		const configPath = `/home/demo/${"nested-folder".repeat(35)}/.claude`;
		await page.setViewportSize({ width, height: width === 1200 ? 800 : 600 });
		await page.addInitScript((path) => {
			Object.defineProperty(window, "__TAURI_INTERNALS__", {
				value: {
					invoke: async () => [
						{
							kind: "claude-code",
							name: "Claude Code",
							detected: true,
							configPath: path,
						},
					],
				},
			});
		}, configPath);
		await page.goto("/");
		const row = page
			.getByRole("list", { name: "Agent inventory" })
			.getByRole("listitem");
		await expect(row.getByText("Detected", { exact: true })).toBeVisible();
		const path = row.getByText(configPath, { exact: true });
		await expect(path).toBeVisible();
		expect(
			await path.evaluate((node) => Array.from(node.getClientRects()).length),
		).toBeGreaterThan(1);
		expect(
			await page.evaluate(() => document.documentElement.scrollWidth),
		).toBeLessThanOrEqual(width);
	});
}

test("opens the palette by button and shortcut and restores focus on Escape", async ({
	page,
}) => {
	await page.goto("/");
	const trigger = page.getByRole("button", { name: /Search/ });
	const dialog = page.getByRole("dialog", { name: "Command palette" });
	await trigger.click();
	await expect(dialog.getByRole("combobox")).toBeFocused();
	await page.keyboard.press("Escape");
	await expect(dialog).toBeHidden();
	await expect(trigger).toBeFocused();

	const search = page.getByRole("searchbox", {
		name: "Search by agent or path",
	});
	await search.focus();
	await page.keyboard.press("Control+k");
	await expect(dialog.getByRole("combobox")).toBeFocused();
	await page.keyboard.press("Escape");
	await expect(search).toBeFocused();
});

test("shows the sample diff alongside the editor in a wide window", async ({
	page,
}) => {
	await page.setViewportSize({ width: 1200, height: 800 });
	await page.goto("/");
	await page.getByRole("link", { name: "Diff demo" }).click();
	await expect(page.getByText(/No local files are changed/)).toBeVisible();
	await expect(
		page.getByRole("textbox", { name: "Sample editor" }),
	).toBeVisible();
	await expect(
		page.getByRole("region", { name: "Changes from original" }),
	).toBeVisible();
	await expect(page.getByRole("button", { name: /save|sync/i })).toHaveCount(0);
});

test("switches diff tabs by keyboard without losing a draft and resets the sample", async ({
	page,
}) => {
	await page.setViewportSize({ width: 900, height: 600 });
	await page.goto("/#/editor");
	const editorTab = page.getByRole("tab", { name: "Sample editor" });
	const diffTab = page.getByRole("tab", { name: "Changes from original" });
	const editor = page.getByRole("textbox", { name: "Sample editor" });
	const reset = page.getByRole("button", { name: "Reset sample" });
	await expect(reset).toBeDisabled();
	await editor.fill("");
	await page.keyboard.insertText('{"edited":true}');
	await expect(editor).toHaveText('{"edited":true}');
	await expect(editor).toBeFocused();
	await editorTab.focus();
	await page.keyboard.press("ArrowRight");
	await expect(diffTab).toBeFocused();
	await expect(diffTab).toHaveAttribute("aria-selected", "true");
	await expect(editor).toBeHidden();
	await expect(
		page.getByText('+ {"edited":true}', { exact: true }),
	).toBeVisible();
	await page.keyboard.press("ArrowLeft");
	await expect(editor).toHaveText('{"edited":true}');
	await reset.click();
	await expect(editor).toContainText("/home/demo");
	await expect(reset).toBeDisabled();
});
