import { Button } from "@/components/ui/button";
import type { LibraryItem, SyncSelection } from "@/lib/library";

const statuses = {
	"in-sync": "In sync",
	drifted: "Drifted",
	"not-linked": "Not linked",
	"broken-symlink": "Broken symlink",
};
export function LinkedTargets({
	item,
	onInspect,
	onUnlink,
}: {
	item: LibraryItem;
	onInspect: (selection: SyncSelection[]) => void;
	onUnlink: (path: string) => void;
}) {
	return (
		<div>
			<h3 className="font-medium">Linked targets</h3>
			{item.targets.length === 0 ? (
				<p className="text-sm text-muted-foreground">
					Not linked to any targets.
				</p>
			) : (
				<ul className="divide-y divide-border">
					{item.targets.map((target) => (
						<li
							key={target.path}
							className="flex flex-wrap items-center justify-between gap-2 py-2"
						>
							<span className="text-sm">
								{target.path} · {statuses[target.status]}
							</span>
							<div className="flex gap-2">
								{(target.status === "drifted" ||
									target.status === "broken-symlink") && (
									<Button
										variant="outline"
										size="sm"
										onClick={() => {
											onInspect([
												{
													agent: target.agent,
													scope: target.scope,
													projectId: target.projectId,
												},
											]);
										}}
									>
										Inspect / repair
									</Button>
								)}
								<Button
									variant="outline"
									size="sm"
									onClick={() => onUnlink(target.path)}
								>
									Unlink
								</Button>
							</div>
						</li>
					))}
				</ul>
			)}
		</div>
	);
}
