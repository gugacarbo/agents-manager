import { Button } from "@/components/ui/button";
export function ConflictBanner({
	busy,
	onReload,
}: {
	busy: boolean;
	onReload: (review: boolean) => void;
}) {
	return (
		<div role="alert" className="space-y-2 rounded-lg border border-border p-3">
			<p>This file changed on disk. Review the latest version before saving.</p>
			<div className="flex flex-wrap gap-2">
				<Button
					variant="outline"
					disabled={busy}
					onClick={() => onReload(false)}
				>
					Reload from disk
				</Button>
				<Button
					variant="outline"
					disabled={busy}
					onClick={() => onReload(true)}
				>
					Overwrite (review diff)
				</Button>
			</div>
		</div>
	);
}
