import { describe, expect, it } from "vitest";
import { environments } from "./environments";

describe("SSH errors", () => {
	it("provides host identity guidance without accepting keys", () => {
		expect(
			environments.message({
				SshError: {
					kind: "host-key",
					message: "Host key verification failed.",
				},
			}),
		).toMatch(/verify and approve.*OpenSSH outside this app/i);
	});
});
