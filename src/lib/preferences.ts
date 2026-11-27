import { z } from "zod";
import { commands } from "../bindings/tauri";
import { isTauriRuntime } from "./agents";
import { formatCoreError } from "./ipc-errors";

const appPreferencesSchema = z.object({
	minimizeToTray: z.boolean(),
});

export type AppPreferences = z.infer<typeof appPreferencesSchema>;

export const appPreferencesQueryKey = ["app-preferences"] as const;

export async function getAppPreferences(): Promise<AppPreferences> {
	if (!isTauriRuntime()) {
		return { minimizeToTray: true };
	}

	try {
		const raw = await commands.getAppPreferences();
		return appPreferencesSchema.parse(raw);
	} catch (err) {
		throw formatCoreError(err);
	}
}

export async function setAppPreferences(
	preferences: AppPreferences,
): Promise<AppPreferences> {
	if (!isTauriRuntime()) {
		return appPreferencesSchema.parse(preferences);
	}

	try {
		const raw = await commands.setAppPreferences(preferences);
		return appPreferencesSchema.parse(raw);
	} catch (err) {
		throw formatCoreError(err);
	}
}
