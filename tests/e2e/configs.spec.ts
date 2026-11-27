import { expect, test } from "@playwright/test";

for (const width of [1200, 900]) {
	test(`edits a synthetic configuration at ${width}px`, async ({ page }) => {
		await page.setViewportSize({ width, height: width === 1200 ? 800 : 600 });
		await page.addInitScript(() => {
			const entry = {
				agent: "codex",
				category: "settings",
				label: "Configuration",
				path: "/synthetic/config.toml",
				format: "toml",
				exists: true,
			};
			let content = 'a = "original"\napi_key = "fixture-value"';
			let sha256 = "hash";
			Object.defineProperty(window, "__TAURI_INTERNALS__", {
				value: {
					transformCallback: () => 1,
					invoke: async (
						command: string,
						args?: { request: { content: string; expectedSha256: string } },
					) => {
						switch (command) {
							case "list_agents":
								return [
									{
										kind: "codex",
										name: "Codex",
										detected: true,
										configPath: "/synthetic",
									},
								];
							case "list_agent_configs":
								return [entry];
							case "read_config_file":
								return {
									entry,
									content,
									sha256,
									secretSpans: [
										{
											startByte: content.indexOf("fixture-value"),
											endByte:
												content.indexOf("fixture-value") +
												"fixture-value".length,
											kind: "api_key",
										},
									],
								};
							case "plugin:event|listen":
								return 1;
							case "plugin:event|unlisten":
							case "watch_config_file":
								return null;
							case "save_config_file":
								if (
									!args ||
									args.request.expectedSha256 !== sha256 ||
									!args.request.content.includes("fixture-value")
								) {
									throw new Error("Unexpected save request");
								}
								content = args.request.content;
								sha256 = "new";
								return { sha256, backupPath: "/synthetic/backup" };
							default:
								throw new Error(`Unexpected command: ${command}`);
						}
					},
				},
			});
		});
		await page.goto("/");
		await page.getByRole("link", { name: "Codex" }).click();
		await page
			.getByRole("button", { name: /Configuration.*config.toml/ })
			.click();
		const editor = page.getByRole("textbox", { name: "Configuration editor" });
		await expect(editor).not.toContainText("fixture-value");
		await editor.press("Home");
		await page.keyboard.insertText("# Draft\n");
		await expect(page.getByRole("button", { name: "Save" })).toBeEnabled();
		if (width === 900) {
			const editorTab = page.getByRole("tab", { name: "Editor" });
			await editorTab.focus();
			await page.keyboard.press("ArrowRight");
			await expect(page.getByRole("tab", { name: "Diff" })).toBeFocused();
			await page.keyboard.press("ArrowLeft");
			await expect(editor).toContainText("# Draft");
		} else
			await expect(
				page.getByRole("region", { name: "Configuration diff viewer" }),
			).toBeVisible();
		await page.getByRole("button", { name: "Save" }).click();
		await expect(page.getByRole("button", { name: "Save" })).toBeDisabled();
		await expect(page.getByRole("alert")).toHaveCount(0);
		await expect(editor).not.toContainText("fixture-value");
	});
}
