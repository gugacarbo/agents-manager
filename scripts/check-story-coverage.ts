import fs from "node:fs";
import path from "node:path";

const ROOT = process.cwd();
const COMPONENTS_DIR = path.join(ROOT, "src", "components");
const PATTERNS_DIR = path.join(COMPONENTS_DIR, "patterns");

const EXCLUDED_FILES = new Set(["index.ts", "index.tsx", "theme-provider.tsx"]);

function toPosix(filePath: string): string {
	return filePath.split(path.sep).join("/");
}

function isExcludedFileName(fileName: string): boolean {
	return (
		EXCLUDED_FILES.has(fileName) || /\.(?:test|stories)\.tsx$/.test(fileName)
	);
}

function collectDirectTsxFiles(dir: string): string[] {
	if (!fs.existsSync(dir)) {
		return [];
	}
	const results: string[] = [];
	for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
		if (
			entry.isFile() &&
			entry.name.endsWith(".tsx") &&
			!isExcludedFileName(entry.name)
		) {
			results.push(path.join(dir, entry.name));
		}
	}
	return results.sort();
}

function collectPatternTsxFiles(dir: string): string[] {
	if (!fs.existsSync(dir)) {
		return [];
	}
	const results: string[] = [];
	for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
		const fullPath = path.join(dir, entry.name);
		if (entry.isDirectory()) {
			results.push(...collectPatternTsxFiles(fullPath));
		} else if (
			entry.isFile() &&
			entry.name.endsWith(".tsx") &&
			!isExcludedFileName(entry.name)
		) {
			results.push(fullPath);
		}
	}
	return results.sort();
}

function main(): void {
	const files = [
		...collectDirectTsxFiles(COMPONENTS_DIR),
		...collectPatternTsxFiles(PATTERNS_DIR),
	];

	const violations: string[] = [];
	for (const absPath of files) {
		const storyPath = absPath.replace(/\.tsx$/, ".stories.tsx");
		if (!fs.existsSync(storyPath)) {
			const relComponent = toPosix(path.relative(ROOT, absPath));
			const relStory = toPosix(path.relative(ROOT, storyPath));
			violations.push(
				`${relComponent}: missing sibling Storybook story (${relStory}).`,
			);
		}
	}

	if (violations.length > 0) {
		for (const violation of violations) {
			console.error(violation);
		}
		process.exit(1);
	}
	console.log(`check-story-coverage: ok (${files.length} files checked)`);
}

if (import.meta.main) {
	main();
}
