import { parse } from "smol-toml";
export type EnvVar = {
	key: string;
	value: string;
	isSecret: boolean;
	isEnvRef: boolean;
};
export type Server = {
	name: string;
	transport: "stdio" | "http";
	command?: string;
	args?: string[];
	url?: string;
	env: EnvVar[];
	enabled: boolean;
};
export function envVar(key: string, value: string): EnvVar {
	return {
		key,
		value,
		isSecret: true,
		isEnvRef: /^\$\{[^}]+\}$/.test(value),
	};
}
function object(value: unknown): Record<string, unknown> {
	return typeof value === "object" && value !== null && !Array.isArray(value)
		? (value as Record<string, unknown>)
		: {};
}
export function parseStructured(content: string, format: "json" | "toml") {
	const root = object(format === "json" ? JSON.parse(content) : parse(content));
	const key = format === "json" ? "mcpServers" : "mcp_servers";
	const servers = Object.entries(object(root[key])).map(
		([name, raw]): Server => {
			const server = object(raw);
			return {
				name,
				transport: typeof server.url === "string" ? "http" : "stdio",
				command:
					typeof server.command === "string" ? server.command : undefined,
				args: Array.isArray(server.args)
					? server.args.filter((arg): arg is string => typeof arg === "string")
					: undefined,
				url: typeof server.url === "string" ? server.url : undefined,
				env: Object.entries(object(server.env))
					.filter(([, value]) => typeof value === "string")
					.map(([envKey, value]) => envVar(envKey, value as string)),
				enabled: server.enabled !== false,
			};
		},
	);
	const settings = Object.fromEntries(
		Object.entries(root).filter(([name]) => name !== key),
	);
	return { servers, settings };
}
