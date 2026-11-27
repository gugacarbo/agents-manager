import {
	type FormEvent,
	useCallback,
	useEffect,
	useId,
	useRef,
	useState,
} from "react";
import { type ClaudeAccount, commands } from "@/bindings/tauri";
import { Button } from "@/components/ui/button";
import { accountErrorMessage } from "./account-status";
import { ClaudeAccountProfile } from "./ClaudeAccountProfile";
import { ClaudeAccountQuotas } from "./ClaudeAccountQuotas";

type Profiles =
	| { state: "loading" }
	| { state: "failed"; message: string }
	| { state: "loaded"; items: ClaudeAccount[] };

export function ClaudeAccountProfiles() {
	const displayNameId = useId();
	const profilesHeadingId = useId();
	const [profiles, setProfiles] = useState<Profiles>({ state: "loading" });
	const [displayName, setDisplayName] = useState("");
	const [creating, setCreating] = useState(false);
	const [createError, setCreateError] = useState("");
	const listing = useRef<AbortController | null>(null);
	const adding = useRef(false);

	const load = useCallback(async () => {
		if (listing.current) return;
		const controller = new AbortController();
		listing.current = controller;
		setProfiles({ state: "loading" });
		try {
			const items = await commands.listClaudeAccounts();
			if (!controller.signal.aborted) setProfiles({ state: "loaded", items });
		} catch (error) {
			if (!controller.signal.aborted) {
				setProfiles({
					state: "failed",
					message: accountErrorMessage(
						error,
						"Could not load local profiles. Try again.",
					),
				});
			}
		} finally {
			if (listing.current === controller) listing.current = null;
		}
	}, []);

	useEffect(() => {
		void load();
		return () => {
			listing.current?.abort();
			listing.current = null;
		};
	}, [load]);

	async function addProfile(event: FormEvent<HTMLFormElement>) {
		event.preventDefault();
		const name = displayName.trim();
		if (adding.current || !name || profiles.state !== "loaded") return;
		adding.current = true;
		setCreating(true);
		setCreateError("");
		try {
			const created = await commands.createClaudeAccount(name);
			setProfiles((current) =>
				current.state === "loaded"
					? { state: "loaded", items: [...current.items, created] }
					: current,
			);
			setDisplayName("");
		} catch (error) {
			setCreateError(
				accountErrorMessage(error, "Could not add the profile. Try again."),
			);
		} finally {
			adding.current = false;
			setCreating(false);
		}
	}

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
				onSubmit={(event) => void addProfile(event)}
			>
				<div className="flex flex-wrap items-end gap-3">
					<div className="min-w-0 flex-1 space-y-1">
						<label className="text-sm font-medium" htmlFor={displayNameId}>
							Display name
						</label>
						<input
							id={displayNameId}
							className="min-h-9 w-full rounded-md border border-input bg-background px-3 text-sm focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
							required
							disabled={profiles.state !== "loaded" || creating}
							value={displayName}
							onChange={(event) => setDisplayName(event.target.value)}
						/>
					</div>
					<Button
						type="submit"
						disabled={
							!displayName.trim() || profiles.state !== "loaded" || creating
						}
					>
						{creating ? "Adding profile…" : "Add profile"}
					</Button>
				</div>
				{createError ? (
					<p role="alert" className="text-sm text-destructive">
						{createError}
					</p>
				) : null}
			</form>
			<section aria-labelledby={profilesHeadingId} className="space-y-3">
				<h2 id={profilesHeadingId} className="text-lg font-semibold">
					Local profiles
				</h2>
				{profiles.state === "loading" ? (
					<p role="status" className="text-sm text-muted-foreground">
						Loading profiles…
					</p>
				) : null}
				{profiles.state === "failed" ? (
					<div className="space-y-2">
						<p role="alert" className="text-sm text-destructive">
							{profiles.message}
						</p>
						<Button type="button" variant="outline" onClick={() => void load()}>
							Retry loading profiles
						</Button>
					</div>
				) : null}
				{profiles.state === "loaded" &&
					(profiles.items.length ? (
						<ul
							aria-labelledby={profilesHeadingId}
							className="divide-y divide-border rounded-lg border border-border bg-card"
						>
							{profiles.items.map((profile) => (
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
