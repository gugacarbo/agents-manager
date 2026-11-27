import { z } from "zod";
import { commands } from "@/bindings/tauri";
import { isTauriRuntime } from "@/lib/agents";
import { formatCoreErrorMessage } from "@/lib/ipc-errors";

const connection = z.discriminatedUnion("mode", [
	z.object({ mode: z.literal("alias"), alias: z.string() }),
	z.object({
		mode: z.literal("manual"),
		host: z.string(),
		user: z.string(),
		port: z.number(),
	}),
]);
const draft = z.object({ name: z.string(), connection });
const environment = draft.extend({ id: z.string() });
const inventory = z.object({
	host: z.string(),
	user: z.string(),
	port: z.number(),
	claude: z.boolean(),
	codex: z.boolean(),
});
export type EnvironmentDraft = z.infer<typeof draft>;
export type SshEnvironment = z.infer<typeof environment>;
export type SshInventory = z.infer<typeof inventory>;
export const environments = {
	list: async () =>
		isTauriRuntime()
			? environment.array().parse(await commands.listSshEnvironments())
			: [],
	save: async (id: string | null, value: EnvironmentDraft) =>
		environment.parse(await commands.saveSshEnvironment(id, value)),
	remove: (id: string) => commands.removeSshEnvironment(id),
	test: async (value: EnvironmentDraft) =>
		inventory.parse(await commands.testSshConnection(value)),
	inspect: async (id: string) =>
		inventory.parse(await commands.inspectSshEnvironment(id)),
	message: (error: unknown) => {
		if (error instanceof Error) return error.message;
		if (typeof error === "string") return error;
		return (
			formatCoreErrorMessage(error) ??
			"Could not connect to the SSH environment."
		);
	},
};
export function validateEnvironment(value: EnvironmentDraft): string | null {
	const valid = (s: string) =>
		s.trim() === s &&
		!!s &&
		!s.startsWith("-") &&
		!/\s/.test(s) &&
		[...s].every(
			(char) => char.charCodeAt(0) > 31 && char.charCodeAt(0) !== 127,
		);
	if (!value.name.trim()) return "Enter a display name.";
	if (value.connection.mode === "alias")
		return valid(value.connection.alias)
			? null
			: "Enter a valid alias without whitespace or a leading hyphen.";
	if (!valid(value.connection.host) || !valid(value.connection.user))
		return "Enter a valid host and user without whitespace or a leading hyphen.";
	if (
		!Number.isInteger(value.connection.port) ||
		value.connection.port < 1 ||
		value.connection.port > 65535
	)
		return "Enter a port between 1 and 65535.";
	return null;
}
