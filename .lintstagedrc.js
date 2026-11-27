/**
 * Validates that lint-staged passed explicit file paths (no globs or directory wildcards).
 * @param {string[]} files
 * @returns {string[]}
 */
export function assertExplicitFiles(files) {
	if (!Array.isArray(files)) {
		throw new Error("lint-staged expected an array of files");
	}
	for (const file of files) {
		if (
			typeof file !== "string" ||
			file.trim().length === 0 ||
			file === "." ||
			file === ".." ||
			file.includes("*") ||
			file.endsWith("/")
		) {
			throw new Error(
				`lint-staged rejected non-explicit path: ${String(file)}`,
			);
		}
	}
	return files;
}

/**
 * Quotes file paths safely for lint-staged command strings.
 * @param {string[]} files
 * @returns {string}
 */
export function quoteFiles(files) {
	const validated = assertExplicitFiles(files);
	return validated
		.map((file) => `"${file.replaceAll("\\", "\\\\").replaceAll('"', '\\"')}"`)
		.join(" ");
}

export default {
	"*.{js,mjs,cjs,ts,tsx}": (files) => {
		if (files.length === 0) return [];
		const quoted = quoteFiles(files);
		return [
			`biome check --write --no-errors-on-unmatched --files-ignore-unknown=true -- ${quoted}`,
			`cspell lint --no-must-find-files --no-progress --no-show-suggestions -- ${quoted}`,
		];
	},
	"*.{json,jsonc,css}": (files) => {
		if (files.length === 0) return [];
		const quoted = quoteFiles(files);
		return `biome check --write --no-errors-on-unmatched --files-ignore-unknown=true -- ${quoted}`;
	},
	"*.{md,yml,yaml,sh}": (files) => {
		if (files.length === 0) return [];
		const quoted = quoteFiles(files);
		return [
			`prettier --write --no-error-on-unmatched-pattern -- ${quoted}`,
			`cspell lint --no-must-find-files --no-progress --no-show-suggestions -- ${quoted}`,
		];
	},
	"*.rs": (files) => {
		if (files.length === 0) return [];
		const quoted = quoteFiles(files);
		return `rustfmt --edition 2024 -- ${quoted}`;
	},
};
