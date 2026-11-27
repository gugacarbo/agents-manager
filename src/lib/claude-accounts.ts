import { z } from "zod";
import {
	type AccountError,
	type AccountStatus_Serialize,
	type ClaudeAccount,
	commands,
	type DesktopQuotaProfile_Serialize,
	type DesktopQuotaRefresh_Serialize,
	type DesktopQuotaState_Serialize,
	type DesktopQuotaWindow_Serialize,
} from "@/bindings/tauri";
import { formatCoreError } from "@/lib/ipc-errors";

export type {
	AccountError,
	AccountStatus_Serialize,
	ClaudeAccount,
	DesktopQuotaProfile_Serialize,
	DesktopQuotaRefresh_Serialize,
	DesktopQuotaState_Serialize,
	DesktopQuotaWindow_Serialize,
};

const claudeAccountSchema = z.object({
	id: z.string(),
	displayName: z.string(),
}) satisfies z.ZodType<ClaudeAccount>;

const accountErrorSchema = z.enum([
	"invalidDisplayName",
	"profileNotFound",
	"storageUnavailable",
	"cliUnavailable",
	"statusTimeout",
	"statusInvalid",
	"terminalUnavailable",
	"loginFailed",
]) satisfies z.ZodType<AccountError>;

const accountStatusSchema = z
	.object({
		profileId: z.string(),
		state: z.enum([
			"pending",
			"connected",
			"disconnected",
			"verificationFailed",
		]),
		errorCode: accountErrorSchema.nullish(),
	})
	.transform(
		({ profileId, state, errorCode }): AccountStatus_Serialize =>
			errorCode == null
				? { profileId, state }
				: { profileId, state, errorCode },
	);

export async function listClaudeAccounts(): Promise<ClaudeAccount[]> {
	try {
		return claudeAccountSchema
			.array()
			.parse(await commands.listClaudeAccounts());
	} catch (err) {
		throw formatCoreError(err);
	}
}

export async function createClaudeAccount(
	displayName: string,
): Promise<ClaudeAccount> {
	try {
		return claudeAccountSchema.parse(
			await commands.createClaudeAccount(displayName),
		);
	} catch (err) {
		throw formatCoreError(err);
	}
}

export async function startClaudeAccountLogin(
	profileId: string,
): Promise<AccountStatus_Serialize> {
	try {
		return accountStatusSchema.parse(
			await commands.startClaudeAccountLogin(profileId),
		);
	} catch (err) {
		throw formatCoreError(err);
	}
}

export async function getClaudeAccountStatus(
	profileId: string,
): Promise<AccountStatus_Serialize> {
	try {
		return accountStatusSchema.parse(
			await commands.getClaudeAccountStatus(profileId),
		);
	} catch (err) {
		throw formatCoreError(err);
	}
}

export async function listClaudeDesktopQuotaProfiles(): Promise<
	DesktopQuotaProfile_Serialize[]
> {
	try {
		return await commands.listClaudeDesktopQuotaProfiles();
	} catch (err) {
		throw formatCoreError(err);
	}
}

export async function createClaudeDesktopQuotaProfile(): Promise<DesktopQuotaProfile_Serialize> {
	try {
		return await commands.createClaudeDesktopQuotaProfile();
	} catch (err) {
		throw formatCoreError(err);
	}
}

export async function openClaudeDesktopQuotaLogin(
	profileId: string,
): Promise<null> {
	try {
		return await commands.openClaudeDesktopQuotaLogin(profileId);
	} catch (err) {
		throw formatCoreError(err);
	}
}

export async function openClaudeDesktopQuotaProfile(
	profileId: string,
): Promise<null> {
	try {
		return await commands.openClaudeDesktopQuotaProfile(profileId);
	} catch (err) {
		throw formatCoreError(err);
	}
}

export async function linkClaudeDesktopQuotaAccount(
	profileId: string,
): Promise<DesktopQuotaProfile_Serialize> {
	try {
		return await commands.linkClaudeDesktopQuotaAccount(profileId);
	} catch (err) {
		throw formatCoreError(err);
	}
}

export async function refreshClaudeDesktopQuota(
	profileId: string,
): Promise<DesktopQuotaRefresh_Serialize> {
	try {
		return await commands.refreshClaudeDesktopQuota(profileId);
	} catch (err) {
		throw formatCoreError(err);
	}
}
