import { z } from "zod";
import { commands } from "@/bindings/tauri";
import { formatCoreError } from "@/lib/ipc-errors";

const projectSchema = z.object({
	id: z.string(),
	name: z.string(),
	rootPath: z.string(),
	addedAt: z.string(),
	available: z.boolean(),
});
export type Project = z.infer<typeof projectSchema>;
export const projects = {
	list: async () => projectSchema.array().parse(await commands.listProjects()),
	register: async (path: string) =>
		projectSchema.parse(await commands.registerProject(path)),
	remove: (id: string) => commands.removeProject(id),
	message: (error: unknown) => formatCoreError(error).message,
};
