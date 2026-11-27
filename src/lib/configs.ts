import { listen } from "@tauri-apps/api/event";
import { z } from "zod";
import { commands } from "@/bindings/tauri";
import { formatCoreError } from "@/lib/ipc-errors";
import { agentKindSchema } from "@/lib/schemas";

const entrySchema = z.object({
	agent: agentKindSchema,
	category: z.enum(["mcps", "skills", "rules", "settings"]),
	label: z.string(),
	path: z.string(),
	format: z.enum(["json", "toml", "markdown"]),
	exists: z.boolean(),
});
const documentSchema = z.object({
	entry: entrySchema,
	content: z.string(),
	sha256: z.string(),
	secretSpans: z.array(
		z.object({ startByte: z.number(), endByte: z.number(), kind: z.string() }),
	),
});
export type ConfigEntry = z.infer<typeof entrySchema>;
export type ConfigDocument = z.infer<typeof documentSchema>;
export const configs = {
	list: async (kind: z.infer<typeof agentKindSchema>) =>
		entrySchema.array().parse(await commands.listAgentConfigs(kind)),
	listForScope: async (
		projectId: string | undefined,
		kind: z.infer<typeof agentKindSchema>,
	) =>
		entrySchema
			.array()
			.parse(
				projectId
					? await commands.listProjectConfigs(projectId, kind)
					: await commands.listAgentConfigs(kind),
			),
	read: async (path: string, entry?: ConfigEntry) => {
		const doc = documentSchema.parse(await commands.readConfigFile(path));
		return entry ? { ...doc, entry } : doc;
	},
	save: (path: string, content: string, expectedSha256: string) =>
		commands.saveConfigFile({ path, content, expectedSha256 }),
	watch: async (path: string, changed: (hash: string | null) => void) => {
		const unlisten = await listen<{ path: string; diskSha256: string | null }>(
			"config-file-changed",
			(event) => {
				if (event.payload.path === path) changed(event.payload.diskSha256);
			},
		);
		try {
			await commands.watchConfigFile(path);
		} catch (error) {
			unlisten();
			throw error;
		}
		return unlisten;
	},
	message: (error: unknown) => formatCoreError(error).message,
};
export function isExternalConflict(error: unknown) {
	return (
		error === "ExternalConflict" ||
		(typeof error === "object" && error !== null && "ExternalConflict" in error)
	);
}

export type StructuredRequest = {
	path: string;
	expectedSha256: string;
	mcpServers?: import("@/lib/structured").Server[];
	settingsPatch?: Record<string, unknown>;
};
export const structuredConfigs = {
	preview: (request: StructuredRequest, draftContent: string) =>
		commands.previewStructuredSave(request, draftContent),
	save: (request: StructuredRequest, draftContent: string) =>
		commands.saveStructuredConfig(request, draftContent),
};
