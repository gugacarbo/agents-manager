import { spawnSync } from "node:child_process";
import path from "node:path";

const GUARD_SCRIPTS = [
	"scripts/check-feature-boundaries.ts",
	"scripts/check-component-size.ts",
	"scripts/check-style-drift.ts",
	"scripts/check-story-coverage.ts",
	"scripts/check-tanstack-conventions.ts",
	"scripts/check-design-system.ts",
];

function runStep(label: string, command: string, args: string[]): boolean {
	console.log(`\n==> ${label}`);
	const result = spawnSync(command, args, {
		cwd: process.cwd(),
		stdio: "inherit",
		env: process.env,
	});
	if (result.error) {
		console.error(`Failed to run "${label}": ${result.error.message}`);
		return false;
	}
	return result.status === 0;
}

function runGuards(): boolean {
	const failed: string[] = [];
	for (const script of GUARD_SCRIPTS) {
		const name = path.basename(script, ".ts");
		if (!runStep(name, "bun", [script])) {
			failed.push(script);
		}
	}
	if (failed.length > 0) {
		console.error(`\nFailed guards (${failed.length}):`);
		for (const script of failed) {
			console.error(`- ${script}`);
		}
		return false;
	}
	return true;
}

function runFast(): boolean {
	const changedBase = process.env.VALIDATE_CHANGED_BASE?.trim();
	const tsTestStep: [string, string, string[]] = changedBase
		? [
				`vitest run --changed ${changedBase}`,
				"bunx",
				["vitest", "run", "--changed", changedBase],
			]
		: ["bun run test:ts", "bun", ["run", "test:ts"]];

	const steps: Array<[string, string, string[]]> = [
		["python3 scripts/docs-check", "python3", ["scripts/docs-check"]],
		["bun run check", "bun", ["run", "check"]],
	];

	for (const [label, command, args] of steps) {
		if (!runStep(label, command, args)) {
			return false;
		}
	}

	if (!runGuards()) {
		return false;
	}

	const trailingSteps: Array<[string, string, string[]]> = [
		["bun run spell", "bun", ["run", "spell"]],
		["bun run knip", "bun", ["run", "knip"]],
		tsTestStep,
	];

	for (const [label, command, args] of trailingSteps) {
		if (!runStep(label, command, args)) {
			return false;
		}
	}

	return true;
}

function runLocal(): boolean {
	if (!runFast()) {
		return false;
	}

	const localSteps: Array<[string, string, string[]]> = [
		["bun run format:check", "bun", ["run", "format:check"]],
		["bun run test", "bun", ["run", "test"]],
		["bun run build:web", "bun", ["run", "build:web"]],
		["bun run test:storybook", "bun", ["run", "test:storybook"]],
		["bun run test:e2e", "bun", ["run", "test:e2e"]],
	];

	for (const [label, command, args] of localSteps) {
		if (!runStep(label, command, args)) {
			return false;
		}
	}

	return true;
}

function main(): void {
	const profile = process.argv[2] ?? "local";
	let ok = false;

	switch (profile) {
		case "guards":
			ok = runGuards();
			break;
		case "fast":
			ok = runFast();
			break;
		case "local":
			ok = runLocal();
			break;
		default:
			console.error(
				`Unknown validation profile "${profile}". Expected one of: guards, fast, local.`,
			);
			process.exit(1);
	}

	if (!ok) {
		process.exit(1);
	}
}

main();
