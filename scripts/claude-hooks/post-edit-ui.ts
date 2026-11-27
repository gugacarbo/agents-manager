import fs from "node:fs";
import path from "node:path";
import { countLines, MAX_COMPONENT_LINES } from "../check-component-size";
import { findStyleDriftViolations } from "../check-style-drift";

interface HookPayload {
	tool_input?: {
		file_path?: unknown;
	};
}

const ROOT = process.cwd();

function toPosix(filePath: string): string {
	return filePath.split(path.sep).join("/");
}

function readHookPayload(): HookPayload | null {
	try {
		const raw = fs.readFileSync(0, "utf8").trim();
		if (!raw) {
			return null;
		}
		return JSON.parse(raw) as HookPayload;
	} catch {
		return null;
	}
}

function isTargetUiFile(relPath: string): boolean {
	if (relPath === "src/router.tsx") {
		return true;
	}
	if (!relPath.endsWith(".tsx")) {
		return false;
	}
	if (relPath.startsWith("src/components/ui/")) {
		return false;
	}
	return (
		relPath.startsWith("src/components/") || relPath.startsWith("src/features/")
	);
}

function isTestOrStory(relPath: string): boolean {
	return /\.(?:test|stories)\.tsx$/.test(relPath);
}

function main(): void {
	const payload = readHookPayload();
	const filePath = payload?.tool_input?.file_path;
	if (typeof filePath !== "string" || filePath.trim() === "") {
		process.exit(0);
	}

	const absPath = path.resolve(ROOT, filePath);
	const relPath = toPosix(path.relative(ROOT, absPath));
	if (!isTargetUiFile(relPath) || !fs.existsSync(absPath)) {
		process.exit(0);
	}

	const content = fs.readFileSync(absPath, "utf8");
	const violations = findStyleDriftViolations(relPath, content);

	if (!isTestOrStory(relPath)) {
		const lines = countLines(content);
		if (lines > MAX_COMPONENT_LINES) {
			violations.push(
				`${relPath}: ${lines} lines exceeds the ${MAX_COMPONENT_LINES}-line limit.`,
			);
		}
	}

	if (violations.length > 0) {
		console.error("UI post-edit guard failed:");
		for (const violation of violations) {
			console.error(`- ${violation}`);
		}
		process.exit(2);
	}

	process.exit(0);
}

main();
