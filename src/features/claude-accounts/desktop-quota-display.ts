const errors: Record<string, string> = {
	desktop_running: "Close Claude Desktop completely, then try again.",
	unauthenticated: "Sign in to Claude Desktop, close it, then try again.",
	source_unavailable:
		"The Claude Desktop quota source is unavailable. Close Claude Desktop and try again.",
	source_incompatible:
		"The Claude Desktop source is incompatible. Update the app and try again.",
	identity_mismatch:
		"The source identity could not be confirmed for the linked account. Check the active Claude Desktop account.",
	rate_limited: "The quota source is rate limited. Wait before trying again.",
	not_linked: "Link the active Claude Desktop account first.",
	duplicate_account: "This account is already linked to another profile.",
	profile_not_found:
		"This profile no longer exists. Reopen the accounts screen.",
	storage_unavailable:
		"Local quota storage is unavailable. Check access to the app data directory.",
};
export function message(error: unknown) {
	return typeof error === "string" && Object.hasOwn(errors, error)
		? errors[error]
		: "Could not access Claude Desktop quota. Check the app and try again.";
}

export type Display = {
	busy?: boolean;
	notice?: string;
	failure?: string;
	current?: boolean;
};
