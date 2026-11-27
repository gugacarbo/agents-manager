import type {
	AccountError,
	AccountStatus_Serialize,
} from "@/lib/claude-accounts";

const accountErrors = {
	invalidDisplayName: "Enter a display name of up to 100 characters.",
	profileNotFound:
		"This profile is no longer available. Reopen Claude accounts to reload the profiles.",
	storageUnavailable:
		"Local profile storage is unavailable. Check that the app can access its data directory and try again.",
	cliUnavailable:
		"Install or update Claude Code to version 2.1.268 or later, then try again.",
	statusTimeout:
		"The status check timed out. Check your connection and try again.",
	statusInvalid:
		"Claude Code did not report a valid profile status. Update Claude Code and try again.",
	terminalUnavailable:
		"Could not open a login terminal. Make sure Ghostty or GNOME Terminal is available, then retry from your Linux desktop.",
	loginFailed:
		"The terminal login did not complete. Retry login and follow the steps in the terminal.",
} satisfies Record<AccountError, string>;

export function accountErrorMessage(error: unknown, fallback: string) {
	const code =
		typeof error === "string"
			? error
			: error instanceof Error
				? error.message
				: null;
	return code !== null && Object.hasOwn(accountErrors, code)
		? accountErrors[code as AccountError]
		: fallback;
}

export type ProfileStatus =
	| { state: "checking" | "pending" | "connected" | "disconnected" }
	| { state: "verificationFailed"; message: string };

export const statusFailure =
	"Could not confirm this profile’s status. Try again.";

export function readStatus(
	result: AccountStatus_Serialize,
	profileId: string,
): ProfileStatus {
	if (result?.profileId === profileId) {
		if (result.state === "verificationFailed") {
			return {
				state: result.state,
				message: accountErrorMessage(result.errorCode, statusFailure),
			};
		}
		if (
			result.errorCode == null &&
			(result.state === "pending" ||
				result.state === "connected" ||
				result.state === "disconnected")
		) {
			return { state: result.state };
		}
	}
	return { state: "verificationFailed", message: statusFailure };
}
