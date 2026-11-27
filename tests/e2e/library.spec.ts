import { expect, test } from "@playwright/test";

test("reviews and applies a library sync", async ({ page }) => {
	await page.addInitScript(() => {
		const item = {
			id: "example",
			category: "rules",
			name: "Example rule",
			sourcePath: "/synthetic/library/example.md",
			hasLiteralSecrets: false,
			targets: [],
		};
		Object.defineProperty(window, "__TAURI_INTERNALS__", {
			value: {
				transformCallback: () => 1,
				invoke: async (command: string, args: Record<string, unknown>) => {
					switch (command) {
						case "list_ssh_environments":
							return [];
						case "list_agents":
							return [];
						case "list_library_items":
							return [item];
						case "list_projects":
							return [];
						case "preview_sync":
							return [
								{
									targetPath: "/synthetic/CLAUDE.md",
									agent: "claude-code",
									scope: "global",
									strategy: "symlink",
									currentContent: "old",
									proposedContent: "new",
									currentSecretSpans: [],
									proposedSecretSpans: [],
									expectedSha256: "old-hash",
									changed: true,
									warnings: [],
								},
							];
						case "apply_sync": {
							const req = args.request as {
								targets: { expectedSha256: string }[];
							};
							if (req.targets[0]?.expectedSha256 !== "old-hash")
								throw Error("Missing reviewed hash");
							return [
								{
									targetPath: "/synthetic/CLAUDE.md",
									applied: true,
									strategyUsed: "symlink",
									backupPath: null,
									error: null,
								},
							];
						}
						default:
							throw Error(`Unexpected command ${command}`);
					}
				},
			},
		});
	});
	await page.goto("/");
	await page.getByRole("link", { name: "Library", exact: true }).click();
	await page.getByRole("button", { name: "Open Example rule" }).click();
	const destination = page.getByRole("checkbox", {
		name: "Claude Code · Global",
		exact: true,
	});
	await destination.focus();
	await page.keyboard.press("Space");
	const previewButton = page.getByRole("button", { name: "Preview sync" });
	await previewButton.focus();
	await page.keyboard.press("Enter");
	await expect(
		page.getByRole("region", { name: "Sync preview" }),
	).toContainText("old-hash");
	await page.getByRole("button", { name: "Apply sync" }).click();
	await expect(page.getByRole("status")).toContainText("Applied (symlink)");
});
