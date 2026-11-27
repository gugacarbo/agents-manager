import fs from "node:fs";
import path from "node:path";

const ROOT = process.cwd();
const SRC_DIR = path.join(ROOT, "src");

const DEPRECATED_QUERY_APIS = [
	"ensureQueryData",
	"fetchQuery",
	"prefetchQuery",
	"ensureInfiniteQueryData",
	"fetchInfiniteQuery",
	"prefetchInfiniteQuery",
];
const DEPRECATED_QUERY_RE = new RegExp(
	`\\b(?:${DEPRECATED_QUERY_APIS.join("|")})\\b`,
);
const TABLE_FEATURES_CALL_RE = /\btableFeatures\s*\(/;
const TOP_LEVEL_TABLE_FEATURES_RE =
	/^(?:export\s+)?const\s+\w+\s*=\s*tableFeatures\s*\(/;

function toPosix(filePath: string): string {
	return filePath.split(path.sep).join("/");
}

function stripCommentsPreservingLines(source: string): string {
	return source
		.replace(/\/\*[\s\S]*?\*\//g, (match) => match.replace(/[^\n]/g, " "))
		.replace(/\/\/[^\n]*/g, "");
}

function collectTsFiles(dir: string): string[] {
	if (!fs.existsSync(dir)) {
		return [];
	}
	const results: string[] = [];
	for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
		const fullPath = path.join(dir, entry.name);
		if (entry.isDirectory()) {
			if (entry.name === "bindings" && dir === SRC_DIR) {
				continue;
			}
			results.push(...collectTsFiles(fullPath));
		} else if (entry.isFile() && /\.(?:ts|tsx)$/.test(entry.name)) {
			results.push(fullPath);
		}
	}
	return results.sort();
}

function checkFile(absPath: string): string[] {
	const relPath = toPosix(path.relative(ROOT, absPath));
	const source = stripCommentsPreservingLines(fs.readFileSync(absPath, "utf8"));
	const lines = source.split(/\r?\n/);
	const violations: string[] = [];

	for (let idx = 0; idx < lines.length; idx++) {
		const line = lines[idx];
		const lineNumber = idx + 1;

		const queryMatch = DEPRECATED_QUERY_RE.exec(line);
		if (queryMatch) {
			violations.push(
				`${relPath}:${lineNumber}: deprecated TanStack Query API "${queryMatch[0]}" is forbidden.`,
			);
		}

		if (
			TABLE_FEATURES_CALL_RE.test(line) &&
			!TOP_LEVEL_TABLE_FEATURES_RE.test(line)
		) {
			violations.push(
				`${relPath}:${lineNumber}: tableFeatures(...) must be declared at module top level ("const ... = tableFeatures(" without indentation).`,
			);
		}
	}

	return violations;
}

function main(): void {
	const files = collectTsFiles(SRC_DIR);
	const violations = files.flatMap(checkFile);
	if (violations.length > 0) {
		for (const violation of violations) {
			console.error(violation);
		}
		process.exit(1);
	}
	console.log(`check-tanstack-conventions: ok (${files.length} files checked)`);
}

if (import.meta.main) {
	main();
}
