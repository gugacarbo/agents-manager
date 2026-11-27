import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import type {
	DesktopConversationResult,
	DesktopQuotaProfile_Serialize,
} from "@/lib/claude-accounts";

export type Conversation = {
	id: string;
	title: string;
	origin: string;
	available: boolean;
	reason: string | null;
};

export function profileName(profile: DesktopQuotaProfile_Serialize) {
	return profile.state.account?.display_name ?? "Default session";
}

/** Opt-in switch for the import; off by default, and nothing else renders until it is on. */
export function ImportToggle({
	checked,
	disabled,
	onChange,
}: {
	checked: boolean;
	disabled: boolean;
	onChange: (checked: boolean) => void;
}) {
	return (
		<div className="space-y-1">
			<div className="flex items-center gap-3 text-sm">
				<Checkbox
					id="conversation-import-enabled"
					checked={checked}
					disabled={disabled}
					onCheckedChange={(next) => onChange(next === true)}
				/>
				<label htmlFor="conversation-import-enabled" className="font-medium">
					Import conversations when switching
				</label>
			</div>
			<p className="text-sm text-muted-foreground">
				Claude Desktop must be closed. Off by default: enabling it only reveals
				a source/destination pair and a listing; nothing is listed or imported
				until you confirm.
			</p>
		</div>
	);
}

export function ConversationPairSelector({
	profiles,
	sourceId,
	destinationId,
	disabled,
	onChange,
}: {
	profiles: DesktopQuotaProfile_Serialize[];
	sourceId: string;
	destinationId: string;
	disabled: boolean;
	onChange: (kind: "source" | "destination", id: string) => void;
}) {
	return (
		<div className="grid gap-3 sm:grid-cols-2">
			{(["source", "destination"] as const).map((kind) => {
				const label =
					kind === "source" ? "Source account" : "Destination account";
				return (
					<label key={kind} className="space-y-1 text-sm">
						<span>{label}</span>
						<select
							aria-label={label}
							className="block h-9 w-full rounded-md border border-input bg-background px-3 text-foreground"
							value={kind === "source" ? sourceId : destinationId}
							disabled={disabled}
							onChange={(event) => onChange(kind, event.target.value)}
						>
							{profiles.map((profile) => (
								<option key={profile.id} value={profile.id}>
									{profileName(profile)}
								</option>
							))}
						</select>
					</label>
				);
			})}
		</div>
	);
}

export function ConversationChoices({
	sessions,
	selectedIds,
	sourceName,
	destinationName,
	importing,
	onToggle,
}: {
	sessions: Conversation[];
	selectedIds: string[];
	sourceName: string;
	destinationName: string;
	importing: boolean;
	onToggle: (id: string, selected: boolean) => void;
}) {
	if (!sessions.length) return null;
	return (
		<fieldset className="space-y-2">
			<legend className="font-medium">
				Conversations from {sourceName} to {destinationName}
			</legend>
			<ul className="divide-y divide-border rounded-md border border-border">
				{sessions.map((session) => {
					const label = `${session.title} from ${session.origin}`;
					return (
						<li key={session.id} className="flex gap-3 p-3">
							<Checkbox
								aria-label={label}
								checked={selectedIds.includes(session.id)}
								disabled={!session.available || importing}
								onCheckedChange={(checked) =>
									onToggle(session.id, checked === true)
								}
							/>
							<div className="min-w-0">
								<p className="font-medium">{session.title}</p>
								<p className="text-sm text-muted-foreground">
									{session.origin}
								</p>
								{!session.available ? (
									<p className="text-sm text-destructive">
										Unavailable:{" "}
										{session.reason ??
											"This conversation cannot be imported yet."}
									</p>
								) : null}
							</div>
						</li>
					);
				})}
			</ul>
		</fieldset>
	);
}

const STATUS_LABEL: Record<DesktopConversationResult["status"], string> = {
	imported: "Imported",
	skipped: "Skipped",
	unavailable: "Unavailable",
};

/** Results grouped by outcome, so imported, skipped (with reason) and unavailable are distinct. */
export function ConversationImportResults({
	results,
	sessions,
	switching,
	importing,
	onRetry,
	onSwitch,
}: {
	results: DesktopConversationResult[];
	sessions: Conversation[];
	switching: boolean;
	importing: boolean;
	onRetry: (ids: string[]) => void;
	onSwitch: () => void;
}) {
	if (!results.length) return null;
	const retryIds = results
		.filter((result) => result.status === "unavailable")
		.map((result) => result.id);
	return (
		<div className="space-y-2">
			<h4 className="font-medium">Import results</h4>
			<ul className="space-y-1">
				{results.map((result) => {
					const session = sessions.find((item) => item.id === result.id);
					const title = session?.title ?? result.id;
					return (
						<li key={result.id} className="text-sm">
							{STATUS_LABEL[result.status]}: {title}
							{result.reason ? ` — ${result.reason}` : ""}
						</li>
					);
				})}
			</ul>
			<ImportActions
				retryIds={retryIds}
				switching={switching}
				importing={importing}
				onRetry={onRetry}
				onSwitch={onSwitch}
			/>
		</div>
	);
}

/** Always offers to continue with the switch; retries only when something is unavailable. */
export function ImportActions({
	retryIds,
	switching,
	importing,
	onRetry,
	onSwitch,
}: {
	retryIds: string[];
	switching: boolean;
	importing: boolean;
	onRetry: (ids: string[]) => void;
	onSwitch: () => void;
}) {
	return (
		<div className="flex flex-wrap gap-2">
			{retryIds.length ? (
				<Button
					type="button"
					variant="outline"
					disabled={importing || switching}
					onClick={() => onRetry(retryIds)}
				>
					Retry unavailable conversations
				</Button>
			) : null}
			<Button
				type="button"
				variant="outline"
				disabled={switching || importing}
				onClick={onSwitch}
			>
				Open destination account
			</Button>
		</div>
	);
}
