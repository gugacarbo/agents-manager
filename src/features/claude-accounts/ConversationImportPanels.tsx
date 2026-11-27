import { Button } from "@/components/ui/button";

/** Listing and cancel. Cancel closes the selection without listing or importing anything. */
export function ImportToolbar({
	loading,
	importing,
	canFind,
	onFind,
	onCancel,
}: {
	loading: boolean;
	importing: boolean;
	canFind: boolean;
	onFind: () => void;
	onCancel: () => void;
}) {
	return (
		<div className="flex flex-wrap gap-2">
			<Button type="button" disabled={!canFind} onClick={onFind}>
				{loading ? "Finding conversations…" : "Find conversations"}
			</Button>
			<Button
				type="button"
				variant="outline"
				onClick={onCancel}
				disabled={loading || importing}
			>
				Cancel
			</Button>
		</div>
	);
}

/** Explicit confirmation before anything is written; the switch itself is separate. */
export function ImportConfirmation({
	count,
	sourceName,
	destinationName,
	importing,
	onConfirm,
	onCancel,
}: {
	count: number;
	sourceName: string;
	destinationName: string;
	importing: boolean;
	onConfirm: () => void;
	onCancel: () => void;
}) {
	return (
		<div
			role="dialog"
			aria-label="Confirm conversation import"
			className="space-y-2 rounded-md border border-border bg-muted p-3"
		>
			<p>
				Import {count} selected conversation{count === 1 ? "" : "s"} from{" "}
				{sourceName} into {destinationName}?
			</p>
			<div className="flex flex-wrap gap-2">
				<Button type="button" disabled={importing} onClick={onConfirm}>
					{importing ? "Importing…" : "Confirm import"}
				</Button>
				<Button
					type="button"
					variant="outline"
					disabled={importing}
					onClick={onCancel}
				>
					Cancel import
				</Button>
			</div>
		</div>
	);
}

/** Origins (local or SSH environments) that could not be checked at all. */
export function ImportOrigins({ origins }: { origins: string[] }) {
	if (!origins.length) return null;
	return (
		<div role="status" className="text-sm text-muted-foreground">
			<p>Unavailable origins:</p>
			<ul className="list-inside list-disc">
				{origins.map((origin) => (
					<li key={origin}>{origin}</li>
				))}
			</ul>
		</div>
	);
}
