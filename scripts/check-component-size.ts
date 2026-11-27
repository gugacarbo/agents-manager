import fs from "node:fs";
import path from "node:path";

export const MAX_COMPONENT_LINES = 250;

const ROOT = process.cwd();

function toPosix(filePath: string): string {
	return filePath.split(path.sep).join("/");
}

function isTestOrStory(relPath: string): boolean {
	return /\.(?:test|stories)\.[^.]+$/.test(relPath);
}

export function isSizeCheckedFile(relPath: string): boolean {
	if (!relPath.endsWith(".tsx") || isTestOrStory(relPath)) {
		return false;
	}
	if (relPath.startsWith("src/components/ui/")) {
		return false;
	}
	return (
		relPath.startsWith("src/components/") || relPath.startsWith("src/features/")
	);
}

export function countLines(content: string): number {
	if (content.length === 0) {
		return 0;
	}
	return content.replace(/\r?\n$/, "").split(/\r?\n/).length;
}

export function checkComponentFileSize(
	relPath: string,
	content: string,
): string | null {
	if (!isSizeCheckedFile(relPath)) {
		return null;
	}
	const lines = countLines(content);
	if (lines > MAX_COMPONENT_LINES) {
		return `${relPath}: ${lines} lines exceeds the ${MAX_COMPONENT_LINES}-line limit.`;
	}
	return null;
}

function collectCheckedFiles(dir: string): string[] {
	if (!fs.existsSync(dir)) {
		return [];
	}
	const results: string[] = [];
	for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
		const fullPath = path.join(dir, entry.name);
		if (entry.isDirectory()) {
			results.push(...collectCheckedFiles(fullPath));
		} else if (entry.isFile()) {
			const relPath = toPosix(path.relative(ROOT, fullPath));
			if (isSizeCheckedFile(relPath)) {
				results.push(fullPath);
			}
		}
	}
	return results.sort();
}

function main(): void {
	const files = [
		...collectCheckedFiles(path.join(ROOT, "src", "components")),
		...collectCheckedFiles(path.join(ROOT, "src", "features")),
	];
	const violations: string[] = [];
	for (const absPath of files) {
		const relPath = toPosix(path.relative(ROOT, absPath));
		const content = fs.readFileSync(absPath, "utf8");
		const violation = checkComponentFileSize(relPath, content);
		if (violation) {
			violations.push(violation);
		}
	}

	if (violations.length > 0) {
		for (const violation of violations) {
			console.error(violation);
		}
		process.exit(1);
	}
	console.log(`check-component-size: ok (${files.length} files checked)`);
}

if (import.meta.main) {
	main();
}
