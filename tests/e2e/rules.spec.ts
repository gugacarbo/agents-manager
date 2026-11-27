import { expect, test } from "@playwright/test";

test("web preview does not expose real rule editing", async ({ page }) => {
	await page.goto("/");
	await expect(page.getByText("Claude Code", { exact: true })).toBeVisible();
	await expect(page.getByRole("link", { name: "Edit rules" })).toHaveCount(0);
	await page.goto("/#/rules");
	await expect(
		page.getByText("Open the desktop app to edit local rules."),
	).toBeVisible();
	await expect(page.getByRole("button", { name: "Save changes" })).toHaveCount(
		0,
	);
});

test("requires a fresh review before saving a changed rule", async ({
	page,
}) => {
	await page.addInitScript(() => {
		Object.defineProperty(window, "__TAURI_INTERNALS__", {
			value: {
				invoke: async (command: string, args: Record<string, string>) => {
					const snapshot = {
						path: "/test-project/CLAUDE.md",
						content: "# Original",
						version: "1",
					};
					switch (command) {
						case "list_ssh_environments":
							return [];
						case "list_agents":
							return [
								{
									kind: "claude-code",
									name: "Claude Code",
									detected: true,
									configPath: "/test/.claude",
								},
							];
						case "open_rule":
							if (args.scope !== "project") throw new Error("wrong scope");
							return snapshot;
						case "review_rule":
							return {
								reviewId: "review",
								original: snapshot.content,
								content: args.content,
							};
						case "save_rule":
							return { ...snapshot, content: "# Final", version: "2" };
						default:
							throw new Error("unexpected IPC");
					}
				},
			},
		});
	});
	await page.goto("/");
	await page.getByRole("link", { name: "Edit rules" }).click();
	await page.getByRole("button", { name: "Choose project folder" }).click();
	await expect(page.getByText("/test-project/CLAUDE.md")).toBeVisible();
	const editor = page.getByRole("textbox", { name: "Claude Code rules" });
	const review = page.getByRole("button", { name: "Review changes" });
	const save = page.getByRole("button", { name: "Save changes" });
	await editor.fill("# Draft");
	await expect(save).toHaveCount(0);
	await review.click();
	await expect(save).toBeVisible();
	await editor.fill("# Final");
	await expect(save).toHaveCount(0);
	await review.click();
	await save.click();
	await expect(page.getByText("Rules saved.")).toBeVisible();
});
