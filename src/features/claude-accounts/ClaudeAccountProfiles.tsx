import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { type FormEvent, useId, useState } from "react";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
	type ClaudeAccount,
	createClaudeAccount,
	listClaudeAccounts,
} from "@/lib/claude-accounts";
import { accountErrorMessage } from "./account-status";
import { ClaudeAccountProfile } from "./ClaudeAccountProfile";
import { ClaudeAccountQuotas } from "./ClaudeAccountQuotas";

const claudeAccountsQueryKey = ["claude-accounts"] as const;

export function ClaudeAccountProfiles() {
	const displayNameId = useId();
	const profilesHeadingId = useId();
	const queryClient = useQueryClient();
	const [displayName, setDisplayName] = useState("");

	const profilesQuery = useQuery({
		queryKey: claudeAccountsQueryKey,
		queryFn: listClaudeAccounts,
	});

	const createMutation = useMutation({
		mutationFn: createClaudeAccount,
		onSuccess: (created) => {
			queryClient.setQueryData<ClaudeAccount[]>(
				claudeAccountsQueryKey,
				(current) => (current ? [...current, created] : [created]),
			);
			setDisplayName("");
		},
	});

	function addProfile(event: FormEvent<HTMLFormElement>) {
		event.preventDefault();
		const name = displayName.trim();
		if (createMutation.isPending || !name || !profilesQuery.isSuccess) return;
		createMutation.reset();
		createMutation.mutate(name);
	}

	const loading =
		profilesQuery.isPending ||
		(profilesQuery.isFetching && !profilesQuery.isSuccess);
	const failed = profilesQuery.isError && !profilesQuery.isFetching;
	const loaded = profilesQuery.isSuccess;
	const items = profilesQuery.data ?? [];
	const createError = createMutation.isError
		? accountErrorMessage(
				createMutation.error,
				"Could not add the profile. Try again.",
			)
		: "";

	return (
		<div className="max-w-3xl space-y-6">
			<header className="space-y-2">
				<h1 className="text-2xl font-semibold tracking-tight">
					Claude Code accounts
				</h1>
				<p className="text-sm text-muted-foreground">
					Claude Code profile management does not use Claude Desktop or change
					your default Claude Code login. Follow the terminal and browser
					prompts to complete the official Claude Code login. The app only
					checks the result.
				</p>
			</header>
			<ClaudeAccountQuotas />
			<form
				className="space-y-2 rounded-lg border border-border bg-card p-4"
				onSubmit={addProfile}
			>
				<div className="flex flex-wrap items-end gap-3">
					<div className="min-w-0 flex-1 space-y-1.5">
						<Label htmlFor={displayNameId}>Display name</Label>
						<Input
							id={displayNameId}
							required
							disabled={!loaded || createMutation.isPending}
							value={displayName}
							onChange={(event) => setDisplayName(event.target.value)}
						/>
					</div>
					<Button
						type="submit"
						disabled={
							!displayName.trim() || !loaded || createMutation.isPending
						}
					>
						{createMutation.isPending ? "Adding profile…" : "Add profile"}
					</Button>
				</div>
				{createError ? (
					<Alert variant="destructive">
						<AlertDescription>{createError}</AlertDescription>
					</Alert>
				) : null}
			</form>
			<section aria-labelledby={profilesHeadingId} className="space-y-3">
				<h2 id={profilesHeadingId} className="text-lg font-semibold">
					Local profiles
				</h2>
				{loading ? (
					<p role="status" className="text-sm text-muted-foreground">
						Loading profiles…
					</p>
				) : null}
				{failed ? (
					<div className="space-y-2">
						<Alert variant="destructive">
							<AlertDescription>
								{accountErrorMessage(
									profilesQuery.error,
									"Could not load local profiles. Try again.",
								)}
							</AlertDescription>
						</Alert>
						<Button
							type="button"
							variant="outline"
							onClick={() =>
								void profilesQuery.refetch({ cancelRefetch: false })
							}
						>
							Retry loading profiles
						</Button>
					</div>
				) : null}
				{loaded &&
					(items.length ? (
						<ul
							aria-labelledby={profilesHeadingId}
							className="divide-y divide-border rounded-lg border border-border bg-card"
						>
							{items.map((profile) => (
								<ClaudeAccountProfile key={profile.id} profile={profile} />
							))}
						</ul>
					) : (
						<p className="text-sm text-muted-foreground">
							No profiles yet. Add a profile to get started.
						</p>
					))}
			</section>
		</div>
	);
}
