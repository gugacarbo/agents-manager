import { Button } from "@/components/ui/button";
import type { RemoteEntry } from "@/lib/remote-configs";
export function RemoteFileList({
	entries,
	listing,
	busy,
	onOpen,
}: {
	entries: RemoteEntry[];
	listing: boolean;
	busy: boolean;
	onOpen: (entry: RemoteEntry) => void;
}) {
	return (
		<>
			<h2 className="font-semibold">Remote global configurations</h2>
			{entries.length ? (
				<div className="flex flex-wrap gap-2">
					{entries.map((entry) => (
						<Button
							variant="outline"
							key={entry.id}
							disabled={busy}
							onClick={() => onOpen(entry)}
						>
							{entry.label}
						</Button>
					))}
				</div>
			) : (
				<p className="text-sm text-muted-foreground">
					{listing ? "Loading remote files…" : "No supported files found."}
				</p>
			)}
		</>
	);
}
