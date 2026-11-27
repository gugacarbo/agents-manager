import { parse } from "smol-toml";

export function validateConfig(
	content: string,
	format: "json" | "toml" | "markdown",
): string | null {
	try {
		if (format === "json") JSON.parse(content);
		if (format === "toml") parse(content);
		return null;
	} catch {
		return "Check the document syntax before saving.";
	}
}
