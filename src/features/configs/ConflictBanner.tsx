import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
export function ConflictBanner({
	busy,
	onReload,
}: {
	busy: boolean;
	onReload: (review: boolean) => void;
}) {
	return (
		<Alert variant="destructive">
			<AlertTitle>File changed on disk</AlertTitle>
			<AlertDescription className="w-full space-y-2">
				<p>
					This file changed on disk. Review the latest version before saving.
				</p>
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
			</AlertDescription>
		</Alert>
	);
}
