import { describe, expect, it } from "vitest";
import { parseStructured } from "./structured";

describe("structured configuration", () => {
	it("reads TOML servers and recognizes literal secrets and environment references", () => {
		const model = parseStructured(
			`[mcp_servers.demo]\ncommand = "node"\nargs = ["app.js"]\n[mcp_servers.demo.env]\nTOKEN = "literal"\nKEY = "\${KEY}"`,
			"toml",
		);
		expect(model.servers[0]).toMatchObject({
			name: "demo",
			command: "node",
			transport: "stdio",
			args: ["app.js"],
		});
		expect(model.servers[0]?.env).toEqual([
			{ key: "TOKEN", value: "literal", isSecret: true, isEnvRef: false },
			{ key: "KEY", value: `${"$"}{KEY}`, isSecret: true, isEnvRef: true },
		]);
	});
});
