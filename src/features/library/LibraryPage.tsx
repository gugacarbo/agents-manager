import { useCallback, useEffect, useState } from "react";
import { PageHeader } from "@/components/patterns";
import { Button } from "@/components/ui/button";
import { type LibraryItem, library } from "@/lib/library";
import { LibraryEditor } from "./LibraryEditor";
import { LibrarySync } from "./LibrarySync";
export function LibraryPage() {
	const [items, setItems] = useState<LibraryItem[]>([]);
	const [loading, setLoading] = useState(true);
	const [error, setError] = useState("");
	const [selected, setSelected] = useState<LibraryItem | null>(null);
	const [editing, setEditing] = useState(false);
	const refresh = useCallback(async () => {
		try {
			const next = await library.list();
			setItems(next);
			setSelected((old) =>
				old ? (next.find((item) => item.id === old.id) ?? null) : null,
			);
			setError("");
		} catch (err) {
			setError(library.message(err));
		} finally {
			setLoading(false);
		}
	}, []);
	useEffect(() => {
		void refresh();
	}, [refresh]);
	return (
		<div className="space-y-5">
			<PageHeader
				title="Library"
				description="Optionally reuse configuration across agents and projects."
				actions={
					<Button
						onClick={() => {
							setSelected(null);
							setEditing(true);
						}}
					>
						Create item
					</Button>
				}
			/>
			{error && (
				<p role="alert" className="text-destructive">
					{error}
				</p>
			)}
			{loading ? (
				<p role="status">Loading library…</p>
			) : items.length === 0 ? (
				<p className="rounded-lg border border-border bg-card p-4 text-sm text-muted-foreground">
					No library items yet. Create an item or promote a native configuration
					to get started.
				</p>
			) : (
				<ul className="divide-y divide-border rounded-lg border border-border bg-card">
					{items.map((item) => (
						<li
							key={item.id}
							className="flex flex-wrap items-center justify-between gap-2 p-3"
						>
							<div>
								<p className="font-medium">{item.name}</p>
								<p className="text-xs text-muted-foreground">
									{item.category} · {item.sourcePath}
								</p>
							</div>
							<Button
								variant="outline"
								onClick={() => {
									setSelected(item);
									setEditing(false);
								}}
							>
								Open {item.name}
							</Button>
						</li>
					))}
				</ul>
			)}
			{editing ? (
				<LibraryEditor
					key={selected?.id ?? "new"}
					item={selected}
					onClose={() => setEditing(false)}
					onSaved={async () => {
						setEditing(false);
						await refresh();
					}}
				/>
			) : (
				selected && (
					<LibrarySync
						key={selected.id}
						item={selected}
						onEdit={() => setEditing(true)}
						onRefresh={refresh}
						onClose={() => setSelected(null)}
					/>
				)
			)}
		</div>
	);
}
