import { expect, test } from "@playwright/test";

test("registers a project and previews a structured edit by keyboard", async ({
	page,
}) => {
	await page.addInitScript(() => {
		const entry = {
			agent: "codex",
			category: "mcps",
			label: "MCP config",
			path: "/test/project/.codex/config.toml",
			format: "toml",
			exists: true,
		};
		const project = {
			id: "project-1",
			name: "Project",
			rootPath: "/test/project",
			addedAt: "today",
			available: true,
		};
		let registered = false;
		let content = '[mcp_servers.demo]\ncommand = "node"';
		let sha256 = "original";
		Object.defineProperty(window, "__TAURI_INTERNALS__", {
			value: {
				transformCallback: () => 1,
				invoke: async (command: string, args: Record<string, unknown>) => {
					switch (command) {
						case "list_ssh_environments":
							return [];
						case "list_agents":
							return [];
						case "list_projects":
							return registered ? [project] : [];
						case "register_project":
							registered = true;
							return project;
						case "list_project_configs":
							return [entry];
						case "read_config_file":
							return {
								entry,
								content,
								sha256,
								secretSpans: [],
							};
						case "plugin:event|listen":
							return 1;
						case "plugin:event|unlisten":
						case "watch_config_file":
							return null;
						case "preview_structured_save": {
							const request = args.request as {
								mcpServers: { command: string }[];
							};
							return {
								content: `[mcp_servers.demo]\ncommand = "${request.mcpServers[0]?.command}"`,
								secretSpans: [],
							};
						}
						case "save_structured_config": {
							const request = args.request as {
								expectedSha256: string;
								mcpServers: { command: string }[];
							};
							if (
								request.expectedSha256 !== sha256 ||
								request.mcpServers[0]?.command !== "bun"
							) {
								throw new Error("Unexpected structured save request");
							}
							content = '[mcp_servers.demo]\ncommand = "bun"';
							sha256 = "saved";
							return { sha256, backupPath: "/test-data/backups/snapshot" };
						}
						default:
							throw new Error(`Unexpected command: ${command}`);
					}
				},
			},
		});
	});
	await page.goto("/");
	await page.getByRole("link", { name: "Projects", exact: true }).click();
	await page.getByLabel("Project folder path").fill("/test/project");
	await page.getByRole("button", { name: "Add project" }).click();
	await page.getByRole("link", { name: "Codex" }).click();
	await page.getByRole("button", { name: /MCP config/ }).click();
	await page.getByRole("button", { name: "Structured" }).click();
	await page.getByLabel("Command", { exact: true }).fill("bun");
	await page.getByRole("button", { name: "Preview changes" }).click();
	await expect(
		page.getByRole("region", { name: "Configuration diff viewer" }),
	).toContainText("bun");
	const save = page.getByRole("button", { name: "Save" });
	await expect(save).toBeEnabled();
	await save.click();
	await expect(
		page.getByRole("textbox", { name: "Configuration editor" }),
	).toContainText('command = "bun"');
	await expect(save).toBeDisabled();
	await expect(page.getByRole("alert")).toHaveCount(0);
});
