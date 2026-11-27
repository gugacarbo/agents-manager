import fs from "node:fs";
import path from "node:path";

const ROOT = process.cwd();

const INLINE_STYLE_RE = /\bstyle=\{\{/;
const ARBITRARY_TAILWIND_RE =
	/\b(?:bg|text|border|w|h|min-w|min-h|max-w|max-h|p|px|py|pt|pb|pl|pr|m|mx|my|gap)-\[[^\]]+\]/;
const RAW_BUTTON_RE = /<button\b/;

function toPosix(filePath: string): string {
	return filePath.split(path.sep).join("/");
}

export function isStyleDriftCheckedFile(relPath: string): boolean {
	if (!/\.(?:ts|tsx)$/.test(relPath)) {
		return false;
	}
	return relPath === "src/router.tsx" || relPath.startsWith("src/features/");
}

export function findStyleDriftViolations(
	relPath: string,
	content: string,
): string[] {
	const violations: string[] = [];
	const lines = content.split(/\r?\n/);

	for (let idx = 0; idx < lines.length; idx++) {
		const line = lines[idx];
		const lineNumber = idx + 1;

		if (INLINE_STYLE_RE.test(line)) {
			violations.push(
				`${relPath}:${lineNumber}: inline style={{...}} is forbidden; use semantic Tailwind classes.`,
			);
		}

		const arbitraryMatch = ARBITRARY_TAILWIND_RE.exec(line);
		if (arbitraryMatch) {
			violations.push(
				`${relPath}:${lineNumber}: arbitrary Tailwind class "${arbitraryMatch[0]}" is forbidden; use design system tokens or scale classes.`,
			);
		}

		if (RAW_BUTTON_RE.test(line)) {
			violations.push(
				`${relPath}:${lineNumber}: raw <button> element is forbidden; use Button from "@/components/ui/button".`,
			);
		}
	}

	return violations;
}

function collectFiles(dir: string): string[] {
	if (!fs.existsSync(dir)) {
		return [];
	}
	const results: string[] = [];
	for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
		const fullPath = path.join(dir, entry.name);
		if (entry.isDirectory()) {
			results.push(...collectFiles(fullPath));
		} else if (entry.isFile() && /\.(?:ts|tsx)$/.test(entry.name)) {
			results.push(fullPath);
		}
	}
	return results.sort();
}

function main(): void {
	const files = collectFiles(path.join(ROOT, "src", "features"));
	const routerPath = path.join(ROOT, "src", "router.tsx");
	if (fs.existsSync(routerPath)) {
		files.push(routerPath);
	}

	const violations: string[] = [];
	for (const absPath of files) {
		const relPath = toPosix(path.relative(ROOT, absPath));
		const content = fs.readFileSync(absPath, "utf8");
		violations.push(...findStyleDriftViolations(relPath, content));
	}

	if (violations.length > 0) {
		for (const violation of violations) {
			console.error(violation);
		}
		process.exit(1);
	}
	console.log(`check-style-drift: ok (${files.length} files checked)`);
}

if (import.meta.main) {
	main();
}
