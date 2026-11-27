import { z } from "zod";
import { commands } from "@/bindings/tauri";
import { formatCoreError } from "@/lib/ipc-errors";

const span = z.object({
	startByte: z.number(),
	endByte: z.number(),
	kind: z.string(),
});
const entry = z.object({
	id: z.enum([
		"claude-settings",
		"claude-home",
		"claude-nested",
		"codex-config",
	]),
	agent: z.enum(["claude-code", "codex"]),
	label: z.string(),
	format: z.enum(["json", "toml"]),
});
const document = z.object({
	handle: z.string().min(1),
	entry,
	host: z.string().min(1),
	user: z.string().min(1),
	port: z.number().int().min(1).max(65535),
	path: z.string().startsWith("/"),
	content: z.string(),
	sha256: z.string(),
	secretSpans: z.array(span),
});
const review = z.object({
	reviewId: z.string().min(1),
	host: z.string().min(1),
	user: z.string().min(1),
	port: z.number().int().min(1).max(65535),
	path: z.string().startsWith("/"),
	originalContent: z.string(),
	proposedContent: z.string(),
	originalSecretSpans: z.array(span),
	proposedSecretSpans: z.array(span),
});
export type RemoteEntry = z.infer<typeof entry>;
export type RemoteDocument = z.infer<typeof document>;
export type RemoteReview = z.infer<typeof review>;
export const remoteConfigs = {
	list: async (environmentId: string, agent: "claude-code" | "codex") =>
		entry
			.array()
			.parse(await commands.listRemoteConfigFiles(environmentId, agent)),
	read: async (environmentId: string, fileId: RemoteEntry["id"]) =>
		document.parse(await commands.openRemoteConfig(environmentId, fileId)),
	review: async (handle: string, content: string) =>
		review.parse(await commands.reviewRemoteConfig(handle, content)),
	save: (reviewId: string) => commands.saveRemoteConfig(reviewId, true),
	verify: async (handle: string) =>
		document.parse(await commands.verifyRemoteConfig(handle)),
	message: (error: unknown) => {
		if (error === "ExternalConflict")
			return "The remote file changed since it was opened.";
		return formatCoreError(error).message;
	},
};

function canonical(value: unknown): unknown {
	if (Array.isArray(value)) return value.map(canonical);
	if (value && typeof value === "object")
		return Object.fromEntries(
			Object.entries(value)
				.sort(([a], [b]) => a.localeCompare(b))
				.map(([key, item]) => [key, canonical(item)]),
		);
	return value;
}
export function protectedProjects(
	original: string,
	draft: string,
	entry: RemoteEntry,
) {
	if (
		entry.agent !== "claude-code" ||
		!["claude-home", "claude-nested"].includes(entry.id)
	)
		return false;
	try {
		return (
			JSON.stringify(canonical(JSON.parse(original).projects)) !==
			JSON.stringify(canonical(JSON.parse(draft).projects))
		);
	} catch {
		return false;
	}
}

export function sameRemoteTarget(
	a: Pick<RemoteDocument, "host" | "user" | "port" | "path">,
	b: Pick<RemoteDocument, "host" | "user" | "port" | "path">,
) {
	return (
		a.host === b.host &&
		a.user === b.user &&
		a.port === b.port &&
		a.path === b.path
	);
}

export function reviewMatches(
	review: RemoteReview,
	doc: RemoteDocument,
	draft: string,
) {
	return (
		sameRemoteTarget(review, doc) &&
		review.originalContent === doc.content &&
		review.proposedContent === draft
	);
}

export function assertRemoteEntry(
	document: RemoteDocument,
	expected: RemoteEntry,
	agent: RemoteEntry["agent"],
) {
	if (
		document.entry.id !== expected.id ||
		document.entry.agent !== agent ||
		document.entry.format !== expected.format
	)
		throw new Error("The selected remote file changed. Open it again.");
}
