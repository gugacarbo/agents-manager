import type { DesktopQuotaRefresh_Serialize } from "@/lib/claude-accounts";

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
	keyring_unavailable:
		"Could not access the Linux keyring. Unlock it and check that secret-tool is installed (libsecret-tools on Ubuntu and Debian).",
	storage_unavailable:
		"Local quota storage is unavailable. Check access to the app data directory.",
};

function quotaErrorCode(error: unknown): string | null {
	if (typeof error === "string") return error;
	if (error instanceof Error) return error.message;
	return null;
}

export function message(error: unknown) {
	const code = quotaErrorCode(error);
	return code !== null && Object.hasOwn(errors, code)
		? errors[code]
		: "Could not access Claude Desktop quota. Check the app and try again.";
}

export function refreshNotice(result: DesktopQuotaRefresh_Serialize): string {
	if (result.status === "wait") {
		return `Please wait ${result.retry_after_seconds ?? 60} seconds before refreshing.`;
	}
	if (result.status === "in_progress") {
		return "A quota check is already in progress.";
	}
	return "";
}

export function actionFailureMessage(
	error: unknown,
	kind: "link" | "open" | "switch",
	isDefault: boolean,
): string {
	const code = quotaErrorCode(error);
	if (kind === "switch" && code === "source_unavailable") {
		return "Could not start Claude Desktop. Check the installation and try again.";
	}
	if (kind === "switch" && code === "unauthenticated") {
		return isDefault
			? "Sign in through the default Claude Desktop window, then try opening this session again."
			: "This profile needs a new sign-in. Use Open login, complete sign-in, then confirm the account.";
	}
	return message(error);
}

export type Display = {
	busy?: boolean;
	notice?: string;
	failure?: string;
	current?: boolean;
};
