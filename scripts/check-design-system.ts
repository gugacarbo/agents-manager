import fs from "node:fs";
import path from "node:path";

const ROOT = process.cwd();
const CSS_PATH = path.join(ROOT, "src", "index.css");
const MAX_CSS_LINES = 160;
const REQUIRED_TOKENS = [
	"--background",
	"--foreground",
	"--card",
	"--border",
	"--primary",
	"--destructive",
];

function countLines(content: string): number {
	if (content.length === 0) {
		return 0;
	}
	return content.replace(/\r?\n$/, "").split(/\r?\n/).length;
}

function main(): void {
	if (!fs.existsSync(CSS_PATH)) {
		console.error("src/index.css: file does not exist.");
		process.exit(1);
	}

	const content = fs.readFileSync(CSS_PATH, "utf8");
	const lines = countLines(content);
	const violations: string[] = [];

	if (lines > MAX_CSS_LINES) {
		violations.push(
			`src/index.css: ${lines} lines exceeds the ${MAX_CSS_LINES}-line limit.`,
		);
	}

	for (const token of REQUIRED_TOKENS) {
		const tokenDecl = new RegExp(`${token}\\s*:`);
		if (!tokenDecl.test(content)) {
			violations.push(
				`src/index.css: missing required design token "${token}".`,
			);
		}
	}

	if (violations.length > 0) {
		for (const violation of violations) {
			console.error(violation);
		}
		process.exit(1);
	}
	console.log(
		`check-design-system: ok (${lines} lines, all ${REQUIRED_TOKENS.length} tokens present)`,
	);
}

if (import.meta.main) {
	main();
}
