import { describe, expect, it } from "vitest";
import { validateConfig } from "./config-validation";

describe("configuration syntax", () => {
	it("rejects invalid JSON and TOML and accepts Markdown", () => {
		expect(validateConfig('{"a":}', "json")).not.toBeNull();
		expect(validateConfig("a = [", "toml")).not.toBeNull();
		expect(validateConfig('a = "ok"', "toml")).toBeNull();
		expect(validateConfig("anything [", "markdown")).toBeNull();
	});
});
