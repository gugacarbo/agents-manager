import { describe, expect, it } from "vitest";
import { projects } from "./projects";

describe("project errors", () => {
	it("explains duplicate projects and points to the existing path", () => {
		expect(
			projects.message({
				DuplicateProject: { id: "1", root_path: "/work/app" },
			}),
		).toContain("/work/app");
		expect(
			projects.message({
				DuplicateProject: { id: "1", root_path: "/work/app" },
			}),
		).toMatch(/already registered/i);
	});
});
