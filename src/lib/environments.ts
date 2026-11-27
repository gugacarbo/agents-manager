import { z } from "zod";
import { commands } from "@/bindings/tauri";
import { isTauriRuntime } from "@/lib/agents";

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
		const typed = z
			.object({ SshError: z.object({ message: z.string(), kind: z.string() }) })
			.safeParse(error);
		if (typed.success) {
			const { kind, message } = typed.data.SshError;
			const hint: Record<string, string> = {
				"host-key":
					"Verify and approve the host key using OpenSSH outside this app.",
				authentication:
					"Configure non-interactive authentication in OpenSSH outside this app.",
				"client-unavailable":
					"Install the OpenSSH client and ensure ssh is in PATH.",
				timeout:
					"Check connectivity and the OpenSSH configuration, then try again.",
				connection:
					"Check the host, port, and OpenSSH configuration, then try again.",
			};
			return [message, hint[kind]].filter(Boolean).join(" ");
		}
		return error instanceof Error
			? error.message
			: typeof error === "string"
				? error
				: "Could not connect to the SSH environment.";
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
