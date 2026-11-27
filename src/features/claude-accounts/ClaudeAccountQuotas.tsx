import { useCallback, useEffect, useRef, useState } from "react";
import { commands, type DesktopQuotaProfile_Serialize } from "@/bindings/tauri";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { DesktopQuotaProfileView } from "./DesktopQuotaProfileView";
import { type Display, message } from "./desktop-quota-display";

type Profile = DesktopQuotaProfile_Serialize;

export function ClaudeAccountQuotas() {
	const [profiles, setProfiles] = useState<Profile[]>([]);
	const [display, setDisplay] = useState<Record<string, Display>>({});
	const [loading, setLoading] = useState(true);
	const [adding, setAdding] = useState(false);
	const [failure, setFailure] = useState("");
	const locks = useRef(new Set<string>());
	const mounted = useRef(false);
	const update = useCallback((id: string, patch: Display) => {
		if (mounted.current)
			setDisplay((previous) => ({
				...previous,
				[id]: { ...previous[id], ...patch },
			}));
	}, []);
	const replace = useCallback((profile: Profile) => {
		if (mounted.current)
			setProfiles((previous) =>
				previous.map((item) => (item.id === profile.id ? profile : item)),
			);
	}, []);
	const refresh = useCallback(
		async (profile: Profile) => {
			if (locks.current.has(profile.id)) return;
			locks.current.add(profile.id);
			update(profile.id, {
				busy: true,
				failure: "",
				notice: "",
				current: false,
			});
			try {
				const result = await commands.refreshClaudeDesktopQuota(profile.id);
				replace({ ...profile, state: result.state });
				update(profile.id, {
					current: result.status === "updated",
					notice:
						result.status === "wait"
							? `Please wait ${result.retry_after_seconds ?? 60} seconds before refreshing.`
							: result.status === "in_progress"
								? "A quota check is already in progress."
								: "",
				});
			} catch (error) {
				update(profile.id, { failure: message(error) });
			} finally {
				locks.current.delete(profile.id);
				update(profile.id, { busy: false });
			}
		},
		[replace, update],
	);
	useEffect(() => {
		mounted.current = true;
		let cancelled = false;
		async function load() {
			try {
				const result = await commands.listClaudeDesktopQuotaProfiles();
				if (cancelled) return;
				setProfiles(result);
				for (const profile of result)
					if (profile.state.account) void refresh(profile);
			} catch (error) {
				if (!cancelled) setFailure(message(error));
			} finally {
				if (!cancelled) setLoading(false);
			}
		}
		void load();
		return () => {
			cancelled = true;
			mounted.current = false;
		};
	}, [refresh]);
	async function action(profile: Profile, kind: "link" | "open") {
		if (locks.current.has(profile.id)) return;
		locks.current.add(profile.id);
		update(profile.id, { busy: true, failure: "", notice: "", current: false });
		try {
			if (kind === "link")
				replace(await commands.linkClaudeDesktopQuotaAccount(profile.id));
			else {
				await commands.openClaudeDesktopQuotaLogin(profile.id);
				update(profile.id, {
					notice:
						"Complete sign-in, close Claude Desktop, then confirm the account.",
				});
			}
		} catch (error) {
			update(profile.id, { failure: message(error) });
		} finally {
			locks.current.delete(profile.id);
			update(profile.id, { busy: false });
		}
	}
	async function add() {
		if (adding) return;
		setAdding(true);
		setFailure("");
		try {
			const profile = await commands.createClaudeDesktopQuotaProfile();
			if (!mounted.current) return;
			setProfiles((previous) => [...previous, profile]);
			await action(profile, "open");
		} catch (error) {
			if (mounted.current) setFailure(message(error));
		} finally {
			if (mounted.current) setAdding(false);
		}
	}
	return (
		<Card aria-label="Claude Desktop quota">
			<CardHeader>
				<CardTitle>Claude Desktop quota</CardTitle>
				<p className="text-sm text-muted-foreground">
					Close Claude Desktop before confirming accounts or refreshing quotas.
				</p>
			</CardHeader>
			<CardContent className="space-y-4">
				<div className="flex flex-wrap gap-2">
					<Button
						type="button"
						disabled={loading || adding}
						onClick={() => void add()}
					>
						Add account
					</Button>
					<Button
						type="button"
						variant="outline"
						disabled={
							loading || !profiles.some((profile) => profile.state.account)
						}
						onClick={() => {
							for (const profile of profiles)
								if (profile.state.account) void refresh(profile);
						}}
					>
						Refresh all
					</Button>
				</div>
				{loading ? (
					<p role="status" className="text-sm">
						Loading Desktop accounts…
					</p>
				) : null}
				{failure ? (
					<p role="alert" className="text-sm text-destructive">
						{failure}
					</p>
				) : null}
				{profiles.map((profile) => (
					<DesktopQuotaProfileView
						key={profile.id}
						profile={profile}
						status={display[profile.id]}
						refresh={refresh}
						action={action}
					/>
				))}
			</CardContent>
		</Card>
	);
}
