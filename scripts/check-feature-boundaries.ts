import fs from "node:fs";
import path from "node:path";

const ROOT = process.cwd();
const SRC_DIR = path.join(ROOT, "src");

function toPosix(filePath: string): string {
	return filePath.split(path.sep).join("/");
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

function stripCommentsPreservingLines(source: string): string {
	return source
		.replace(/\/\*[\s\S]*?\*\//g, (match) => match.replace(/[^\n]/g, " "))
		.replace(/\/\/[^\n]*/g, "");
}

type ImportReference = {
	specifier: string;
	line: number;
	isTypeOnly: boolean;
};

function isTypeOnlyClause(clause: string): boolean {
	const trimmed = clause.trim();
	if (/^type(?:\s+|\{)/.test(trimmed)) {
		return true;
	}
	if (trimmed.startsWith("{") && trimmed.endsWith("}")) {
		const inner = trimmed.slice(1, -1).trim();
		if (inner === "") {
			return true;
		}
		return inner
			.split(",")
			.map((part) => part.trim())
			.filter((part) => part.length > 0)
			.every((part) => /^type\s+/.test(part));
	}
	return false;
}

function extractImportSpecifiers(source: string): ImportReference[] {
	const cleaned = stripCommentsPreservingLines(source);
	const results: ImportReference[] = [];
	const fromPattern =
		/\b(?:import|export)\s+((?:type\s+)?(?:\*(?:\s+as\s+\w+)?|\{[^}]*\}|\w+(?:\s*,\s*(?:\{[^}]*\}|\*\s+as\s+\w+))?))\s*\bfrom\s*["']([^"']+)["']/g;
	for (const match of cleaned.matchAll(fromPattern)) {
		const clause = match[1] ?? "";
		const specifier = match[2];
		const index = match.index ?? 0;
		const line = cleaned.slice(0, index).split("\n").length;
		results.push({ specifier, line, isTypeOnly: isTypeOnlyClause(clause) });
	}

	const valuePatterns = [
		/\bimport\s*["']([^"']+)["']/g,
		/\b(?:import|require)\s*\(\s*["']([^"']+)["']\s*\)/g,
	];

	for (const pattern of valuePatterns) {
		for (const match of cleaned.matchAll(pattern)) {
			const specifier = match[1];
			const index = match.index ?? 0;
			const line = cleaned.slice(0, index).split("\n").length;
			results.push({ specifier, line, isTypeOnly: false });
		}
	}
	return results;
}

function getFeatureName(relPath: string): string | null {
	const match = /^src\/features\/([^/]+)\//.exec(relPath);
	return match ? match[1] : null;
}

function isIpcRestricted(relPath: string): boolean {
	return (
		relPath === "src/router.tsx" ||
		relPath.startsWith("src/components/") ||
		relPath.startsWith("src/features/")
	);
}

function isBindingsTauriSpecifier(specifier: string, absPath: string): boolean {
	if (
		specifier === "@/bindings/tauri" ||
		specifier.startsWith("@/bindings/tauri/") ||
		specifier === "@/bindings" ||
		specifier.startsWith("@/bindings/")
	) {
		return true;
	}
	if (specifier.startsWith(".")) {
		const resolved = toPosix(
			path.relative(ROOT, path.resolve(path.dirname(absPath), specifier)),
		);
		return /^src\/bindings(?:\/|$)/.test(resolved);
	}
	return false;
}

function checkFile(absPath: string): string[] {
	const relPath = toPosix(path.relative(ROOT, absPath));
	const ownFeature = getFeatureName(relPath);
	const source = fs.readFileSync(absPath, "utf8");
	const violations: string[] = [];
	const ipcRestricted = isIpcRestricted(relPath);

	for (const { specifier, line, isTypeOnly } of extractImportSpecifiers(
		source,
	)) {
		const aliasMatch = /^@\/features\/([^/]+)\/(.+)$/.exec(specifier);
		if (aliasMatch) {
			const targetFeature = aliasMatch[1];
			violations.push(
				`${relPath}:${line}: deep feature import "${specifier}" is forbidden; import from "@/features/${targetFeature}" instead.`,
			);
		}

		if (specifier.startsWith(".")) {
			const resolved = toPosix(
				path.relative(ROOT, path.resolve(path.dirname(absPath), specifier)),
			);
			const targetMatch = /^src\/features\/([^/]+)(?:\/|$)/.exec(resolved);
			if (targetMatch) {
				const targetFeature = targetMatch[1];
				if (ownFeature !== targetFeature) {
					violations.push(
						`${relPath}:${line}: relative cross-feature import "${specifier}" is forbidden; import from "@/features/${targetFeature}" instead.`,
					);
				}
			}
		}

		if (
			ipcRestricted &&
			(specifier === "@tauri-apps/api/core" ||
				specifier.startsWith("@tauri-apps/api/core/"))
		) {
			violations.push(
				`${relPath}:${line}: direct import of "${specifier}" is forbidden in UI/router layers; route IPC through src/bindings/ or src/lib/.`,
			);
		}

		if (
			ipcRestricted &&
			!isTypeOnly &&
			isBindingsTauriSpecifier(specifier, absPath)
		) {
			violations.push(
				`${relPath}:${line}: non-type import of "${specifier}" is forbidden in UI/router layers; route IPC through src/lib/.`,
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
	console.log(`check-feature-boundaries: ok (${files.length} files checked)`);
}

if (import.meta.main) {
	main();
}
