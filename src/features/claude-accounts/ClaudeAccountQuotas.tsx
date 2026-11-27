import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { ClaudeDesktopConversationImport } from "./ClaudeDesktopConversationImport";
import { DesktopQuotaProfileView } from "./DesktopQuotaProfileView";
import { useClaudeDesktopQuotas } from "./use-claude-desktop-quotas";

export function ClaudeAccountQuotas() {
	const {
		profiles,
		display,
		loading,
		adding,
		switching,
		failure,
		refresh,
		action,
		add,
	} = useClaudeDesktopQuotas();

	const isolated = profiles.find((profile) => profile.shares_default_quota);

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
							for (const profile of profiles) {
								if (profile.state.account && !profile.shares_default_quota)
									void refresh(profile);
							}
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
				{profiles
					.filter((profile) => !profile.shares_default_quota)
					.map((profile) => (
						<DesktopQuotaProfileView
							key={profile.id}
							profile={profile}
							isolated={profile.is_default ? isolated : undefined}
							isolatedStatus={
								profile.is_default && isolated
									? display[isolated.id]
									: undefined
							}
							status={display[profile.id]}
							switching={switching}
							refresh={refresh}
							action={action}
						/>
					))}
				<ClaudeDesktopConversationImport
					profiles={profiles}
					switching={switching}
					onSwitch={(profile) => action(profile, "switch")}
				/>
			</CardContent>
		</Card>
	);
}
