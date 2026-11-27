import { z } from "zod";
import { commands } from "@/bindings/tauri";

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
	message: (error: unknown) => {
		const duplicate = z
			.object({ DuplicateProject: z.object({ root_path: z.string() }) })
			.safeParse(error);
		if (duplicate.success)
			return `Project already registered: ${duplicate.data.DuplicateProject.root_path}`;
		if (error instanceof Error) return error.message;
		if (typeof error === "string") return error;
		return JSON.stringify(error);
	},
};
