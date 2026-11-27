import { useCallback, useEffect, useId, useRef, useState } from "react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
	type ClaudeAccount,
	getClaudeAccountStatus,
	startClaudeAccountLogin,
} from "@/lib/claude-accounts";
import {
	accountErrorMessage,
	type ProfileStatus,
	readStatus,
	statusFailure,
} from "./account-status";

const statusLabels = {
	checking: "Checking status…",
	pending: "Login in progress",
	connected: "Connected",
	disconnected: "Disconnected",
	verificationFailed: "Verification failed",
};

export function ClaudeAccountProfile({ profile }: { profile: ClaudeAccount }) {
	const { id, displayName } = profile;
	const labelId = useId();
	const operation = useRef<AbortController | null>(null);
	const [status, setStatus] = useState<ProfileStatus>({ state: "checking" });
	const [loginAttempted, setLoginAttempted] = useState(false);

	const run = useCallback(
		async (kind: "check" | "login") => {
			if (operation.current) return;
			const controller = new AbortController();
			const { signal } = controller;
			operation.current = controller;
			if (kind === "login") {
				setLoginAttempted(true);
				setStatus({ state: "pending" });
			} else {
				setStatus({ state: "checking" });
			}

			try {
				const request =
					kind === "login" ? startClaudeAccountLogin : getClaudeAccountStatus;
				let next = readStatus(await request(id), id);
				// A previous visit may have started a terminal login that is still open.
				// Serialize checks and never poll while this visit's start command is in flight.
				while (!signal.aborted && next.state === "pending") {
					setStatus(next);
					await new Promise<void>((resolve) => setTimeout(resolve, 500));
					if (signal.aborted) return;
					next = readStatus(await getClaudeAccountStatus(id), id);
				}
				if (!signal.aborted) setStatus(next);
			} catch (error) {
				if (!signal.aborted) {
					setStatus({
						state: "verificationFailed",
						message: accountErrorMessage(error, statusFailure),
					});
				}
			} finally {
				if (operation.current === controller) operation.current = null;
			}
		},
		[id],
	);

	useEffect(() => {
		void run("check");
		return () => {
			operation.current?.abort();
			operation.current = null;
		};
	}, [run]);

	const checking = status.state === "checking";
	const pending = status.state === "pending";
	const failure = status.state === "verificationFailed";
	const loginLabel = pending
		? "Login in progress"
		: status.state === "connected"
			? "Sign in again"
			: loginAttempted
				? "Retry login"
				: "Start login";
	return (
		<li aria-labelledby={labelId} className="space-y-2 p-4">
			<div className="flex flex-wrap items-center justify-between gap-3">
				<div className="min-w-0 space-y-1">
					<h3 id={labelId} className="wrap-anywhere font-medium">
						{displayName}
					</h3>
					<div role="status">
						<Badge variant={status.state === "connected" ? "success" : "muted"}>
							{statusLabels[status.state]}
						</Badge>
					</div>
				</div>
				<div className="flex flex-wrap gap-2">
					{failure ? (
						<Button
							type="button"
							variant="outline"
							onClick={() => void run("check")}
						>
							Retry status check
						</Button>
					) : null}
					<Button
						type="button"
						variant="outline"
						disabled={checking || pending}
						onClick={() => void run("login")}
					>
						{loginLabel}
					</Button>
				</div>
			</div>
			{failure ? (
				<p role="alert" className="text-sm text-destructive">
					{status.message}
				</p>
			) : null}
			{status.state === "disconnected" && loginAttempted ? (
				<p className="text-sm text-muted-foreground">
					Login was not confirmed. Retry login and complete the steps in the
					terminal.
				</p>
			) : null}
		</li>
	);
}
