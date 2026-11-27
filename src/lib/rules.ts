import { invoke } from "@tauri-apps/api/core";

export type RuleScope = "global" | "project";
export type RuleSnapshot = { path: string; content: string; version: string };
export type RuleReview = {
	reviewId: string;
	original: string;
	content: string;
};

function message(error: unknown): string {
	if (error instanceof Error) return error.message;
	if (typeof error === "string") return error;
	return "The desktop app could not complete this operation.";
}

export const rules = {
	open: (scope: RuleScope) => invoke<RuleSnapshot>("open_rule", { scope }),
	reload: (version: string) => invoke<RuleSnapshot>("reload_rule", { version }),
	review: (version: string, content: string) =>
		invoke<RuleReview>("review_rule", { version, content }),
	save: (reviewId: string) => invoke<RuleSnapshot>("save_rule", { reviewId }),
	message,
};
