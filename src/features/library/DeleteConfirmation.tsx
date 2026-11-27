import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import type { LibraryItem } from "@/lib/library";

const statuses = {
	"in-sync": "In sync",
	drifted: "Drifted",
	"not-linked": "Not linked",
	"broken-symlink": "Broken symlink",
};
export function DeleteConfirmation({
	item,
	onConfirm,
}: {
	item: LibraryItem;
	onConfirm: () => void;
}) {
	return (
		<Alert variant="destructive">
			<AlertTitle>Confirm deletion</AlertTitle>
			<AlertDescription className="w-full space-y-2">
				<p>
					Deleting this item affects these linked targets. Native copies are
					preserved.
				</p>
				<ul>
					{item.targets.map((target) => (
						<li key={target.path} className="font-mono text-xs wrap-anywhere">
							{target.path} · {statuses[target.status]}
						</li>
					))}
				</ul>
				<Button onClick={() => onConfirm()}>Confirm delete</Button>
			</AlertDescription>
		</Alert>
	);
}
