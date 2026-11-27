import { useEffect, useMemo, useState } from "react";
import { Button } from "@/components/ui/button";
import {
	type DesktopConversationResult,
	type DesktopQuotaProfile_Serialize,
	importClaudeDesktopConversations,
	listClaudeDesktopConversations,
} from "@/lib/claude-accounts";
import {
	ImportConfirmation,
	ImportOrigins,
	ImportToolbar,
} from "./ConversationImportPanels";
import {
	type Conversation,
	ConversationChoices,
	ConversationImportResults,
	ConversationPairSelector,
	ImportActions,
	ImportToggle,
	profileName,
} from "./ConversationImportViews";

const LIST_FAILURE =
	"Could not list conversations. Check that Claude Desktop is closed, then retry.";
const IMPORT_FAILURE =
	"Import could not finish. Retry the selected conversations or switch accounts.";

/**
 * Opt-in conversation import for an account switch. Nothing is listed or imported until the
 * user enables it; cancelling leaves the simple switch untouched.
 */
export function ClaudeDesktopConversationImport({
	profiles,
	switching,
	onSwitch,
}: {
	profiles: DesktopQuotaProfile_Serialize[];
	switching: boolean;
	onSwitch: (profile: DesktopQuotaProfile_Serialize) => Promise<void>;
}) {
	const eligibleProfiles = useMemo(
		() => profiles.filter((profile) => profile.state.account),
		[profiles],
	);
	const [enabled, setEnabled] = useState(false);
	const [sourceId, setSourceId] = useState("");
	const [destinationId, setDestinationId] = useState("");
	const [sessions, setSessions] = useState<Conversation[]>([]);
	const [unavailableOrigins, setUnavailableOrigins] = useState<string[]>([]);
	const [selectedIds, setSelectedIds] = useState<string[]>([]);
	const [results, setResults] = useState<DesktopConversationResult[]>([]);
	const [loading, setLoading] = useState(false);
	const [importing, setImporting] = useState(false);
	const [failure, setFailure] = useState("");
	const [confirming, setConfirming] = useState(false);

	useEffect(() => {
		if (eligibleProfiles.length < 2) return;
		if (!eligibleProfiles.some((profile) => profile.id === sourceId)) {
			setSourceId(eligibleProfiles[0].id);
		}
		if (
			!eligibleProfiles.some((profile) => profile.id === destinationId) ||
			destinationId === sourceId
		) {
			setDestinationId(
				eligibleProfiles.find((profile) => profile.id !== sourceId)?.id ??
					eligibleProfiles[1].id,
			);
		}
	}, [eligibleProfiles, sourceId, destinationId]);

	if (eligibleProfiles.length < 2) return null;

	const source = eligibleProfiles.find((profile) => profile.id === sourceId);
	const destination = eligibleProfiles.find(
		(profile) => profile.id === destinationId,
	);
	const pairReady =
		!!source &&
		!!destination &&
		source.id !== destination.id &&
		!loading &&
		!importing;
	const sourceName = source ? profileName(source) : "source";
	const destinationName = destination
		? profileName(destination)
		: "destination";

	function resetSelection() {
		setSessions([]);
		setUnavailableOrigins([]);
		setSelectedIds([]);
		setResults([]);
		setFailure("");
		setConfirming(false);
	}

	function cancel() {
		setEnabled(false);
		resetSelection();
	}

	function changePair(kind: "source" | "destination", id: string) {
		if (kind === "source") setSourceId(id);
		else setDestinationId(id);
		resetSelection();
	}

	async function findConversations() {
		if (!pairReady) return;
		setLoading(true);
		resetSelection();
		try {
			const listing = await listClaudeDesktopConversations(
				source.id,
				destination.id,
			);
			setSessions(listing.sessions);
			setUnavailableOrigins(listing.unavailableOrigins);
		} catch {
			setFailure(LIST_FAILURE);
		}
		setLoading(false);
	}

	async function importSelected(ids: string[] = selectedIds) {
		if (!pairReady || !ids.length) return;
		setImporting(true);
		setFailure("");
		setConfirming(false);
		try {
			setResults(
				await importClaudeDesktopConversations(source.id, destination.id, ids),
			);
		} catch {
			// Per-session results are unavailable; the switch stays available and the import can be retried.
			setFailure(IMPORT_FAILURE);
		}
		setImporting(false);
	}

	function toggle(id: string, selected: boolean) {
		setSelectedIds((current) =>
			selected
				? [...current, id]
				: current.filter((selectedId) => selectedId !== id),
		);
	}

	const openDestination = () =>
		destination ? void onSwitch(destination) : undefined;
	const showActions =
		results.length > 0 || !!failure || unavailableOrigins.length > 0;
	const emptyListing =
		sessions.length === 0 &&
		unavailableOrigins.length === 0 &&
		!loading &&
		!failure;

	return (
		<section
			aria-label="Conversation import"
			className="space-y-3 rounded-lg border border-border p-4"
		>
			<ImportToggle
				checked={enabled}
				disabled={loading || importing}
				onChange={(checked) => (checked ? setEnabled(true) : cancel())}
			/>
			{enabled ? (
				<div className="space-y-3">
					<ConversationPairSelector
						profiles={eligibleProfiles}
						sourceId={sourceId}
						destinationId={destinationId}
						disabled={loading || importing}
						onChange={changePair}
					/>
					<ImportToolbar
						loading={loading}
						importing={importing}
						canFind={pairReady}
						onFind={() => void findConversations()}
						onCancel={cancel}
					/>
					{failure ? (
						<p role="alert" className="text-sm text-destructive">
							{failure}
						</p>
					) : null}
					<ImportOrigins origins={unavailableOrigins} />
					<ConversationChoices
						sessions={sessions}
						selectedIds={selectedIds}
						sourceName={sourceName}
						destinationName={destinationName}
						importing={importing}
						onToggle={toggle}
					/>
					{emptyListing ? (
						<p className="text-sm text-muted-foreground">
							No conversations found for this account pair.
						</p>
					) : null}
					{selectedIds.length ? (
						<Button
							type="button"
							disabled={importing}
							onClick={() => setConfirming(true)}
						>
							Import selected conversations
						</Button>
					) : null}
					{confirming ? (
						<ImportConfirmation
							count={selectedIds.length}
							sourceName={sourceName}
							destinationName={destinationName}
							importing={importing}
							onConfirm={() => void importSelected()}
							onCancel={() => setConfirming(false)}
						/>
					) : null}
					<ConversationImportResults
						results={results}
						sessions={sessions}
						switching={switching}
						importing={importing}
						onRetry={(ids) => void importSelected(ids)}
						onSwitch={openDestination}
					/>
					{!results.length && showActions ? (
						<ImportActions
							retryIds={failure ? selectedIds : []}
							switching={switching}
							importing={importing}
							onRetry={(ids) => void importSelected(ids)}
							onSwitch={openDestination}
						/>
					) : null}
				</div>
			) : null}
		</section>
	);
}
