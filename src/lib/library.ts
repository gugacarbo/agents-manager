import { z } from "zod";
import { commands } from "@/bindings/tauri";
import { formatCoreError } from "@/lib/ipc-errors";
import { agentKindSchema } from "@/lib/schemas";

const category = z.enum(["mcps", "skills", "rules", "settings"]);
const scope = z.enum(["global", "project"]);
const selection = z.object({
	agent: agentKindSchema,
	scope,
	projectId: z.string().nullish(),
	copy: z.boolean().optional(),
});
const target = z.object({
	agent: agentKindSchema,
	scope,
	projectId: z.string().nullish(),
	path: z.string(),
	status: z.enum(["in-sync", "drifted", "not-linked", "broken-symlink"]),
});
const item = z.object({
	id: z.string(),
	category,
	name: z.string(),
	sourcePath: z.string(),
	hasLiteralSecrets: z.boolean(),
	targets: z.array(target),
});
const secretSpan = z.object({
	startByte: z.number(),
	endByte: z.number(),
	kind: z.string(),
});
const preview = z.object({
	currentSecretSpans: z.array(secretSpan),
	proposedSecretSpans: z.array(secretSpan),
	targetPath: z.string(),
	agent: agentKindSchema,
	scope,
	strategy: z.enum(["symlink", "copy", "merge"]),
	currentContent: z.string().nullable(),
	proposedContent: z.string(),
	expectedSha256: z.string().nullable(),
	changed: z.boolean(),
	warnings: z.array(z.string()),
});
export type LibraryItem = z.infer<typeof item>;
export type SyncSelection = z.infer<typeof selection>;
export type SyncPreview = z.infer<typeof preview>;
export type Category = z.infer<typeof category>;
export const library = {
	list: async () => item.array().parse(await commands.listLibraryItems()),
	promote: (request: {
		path: string;
		category: Category;
		name: string;
		confirmLiteralSecrets?: boolean;
	}) => commands.promoteToLibrary(request),
	read: (id: string) => commands.readLibraryItem(id),
	save: (request: {
		itemId?: string;
		category: Category;
		name: string;
		content: string;
		confirmLiteralSecrets?: boolean;
	}) => commands.saveLibraryItem(request),
	delete: (id: string) => commands.deleteLibraryItem(id),
	unlink: (id: string, path: string) => commands.unlinkSync(id, path),
	preview: async (itemId: string, targets: SyncSelection[]) =>
		preview.array().parse(await commands.previewSync({ itemId, targets })),
	apply: (
		itemId: string,
		targets: { selection: SyncSelection; expectedSha256: string | null }[],
	) => commands.applySync({ itemId, targets }),
	message: (error: unknown) => formatCoreError(error).message,
};
