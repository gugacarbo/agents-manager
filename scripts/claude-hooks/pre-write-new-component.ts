import { createHash } from "node:crypto";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";

interface HookPayload {
	session_id?: unknown;
	tool_input?: {
		file_path?: unknown;
	};
}

const ROOT = process.cwd();
const MARKER_DIR = path.join(os.tmpdir(), "agents-manager-pre-write-ui");

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

function isNewComponentCandidate(relPath: string): boolean {
	if (!relPath.endsWith(".tsx")) {
		return false;
	}
	if (/\.(?:test|stories)\.tsx$/.test(relPath)) {
		return false;
	}
	if (relPath.startsWith("src/components/ui/")) {
		return false;
	}
	return (
		relPath.startsWith("src/components/") || relPath.startsWith("src/features/")
	);
}

function getMarkerPath(sessionId: string, relPath: string): string {
	const digest = createHash("sha256")
		.update(`${sessionId}\0${relPath}`)
		.digest("hex");
	return path.join(MARKER_DIR, `${digest}.marker`);
}

function main(): void {
	const payload = readHookPayload();
	const filePath = payload?.tool_input?.file_path;
	if (typeof filePath !== "string" || filePath.trim() === "") {
		process.exit(0);
	}

	const absPath = path.resolve(ROOT, filePath);
	const relPath = toPosix(path.relative(ROOT, absPath));
	if (!isNewComponentCandidate(relPath) || fs.existsSync(absPath)) {
		process.exit(0);
	}

	const sessionId =
		typeof payload?.session_id === "string" && payload.session_id.trim() !== ""
			? payload.session_id.trim()
			: "default";
	const markerPath = getMarkerPath(sessionId, relPath);

	if (fs.existsSync(markerPath)) {
		process.exit(0);
	}

	fs.mkdirSync(MARKER_DIR, { recursive: true });
	fs.writeFileSync(markerPath, `${relPath}\n`, "utf8");

	console.error(
		[
			`Before creating new component "${relPath}", check the existing catalog and hierarchy:`,
			"1. Read .agents/skills/frontend-ui/references/components.generated.md",
			"2. Follow Layout/Route -> Pattern -> Product Component -> shadcn Primitive -> Novo componente",
			"3. Re-run Write if no existing component or pattern covers this need.",
		].join("\n"),
	);
	process.exit(2);
}

main();
