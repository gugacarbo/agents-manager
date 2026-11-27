import { Command } from "cmdk";
import { FileCode2, LayoutGrid, RefreshCw } from "lucide-react";
import { useEffect, useState } from "react";

interface CommandPaletteProps {
	onNavigate: (path: "/" | "/editor") => void;
	onRefreshAgents: () => void;
}

export function CommandPalette({
	onNavigate,
	onRefreshAgents,
}: CommandPaletteProps) {
	const [open, setOpen] = useState(false);

	useEffect(() => {
		function handleKeyDown(event: KeyboardEvent) {
			if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "k") {
				event.preventDefault();
				setOpen((prev) => !prev);
			} else if (event.key === "Escape") {
				setOpen(false);
			}
		}

		window.addEventListener("keydown", handleKeyDown);
		return () => window.removeEventListener("keydown", handleKeyDown);
	}, []);

	if (!open) {
		return null;
	}

	return (
		<div
			role="dialog"
			aria-label="Command palette"
			className="fixed inset-0 z-50 flex items-start justify-center bg-slate-950/70 pt-24 backdrop-blur-xs"
		>
			<Command className="w-full max-w-lg overflow-hidden rounded-xl border border-border bg-card text-card-foreground shadow-2xl">
				<Command.Input
					autoFocus
					placeholder="Type a command or navigate..."
					className="w-full border-b border-border bg-transparent px-4 py-3 text-sm text-foreground outline-none placeholder:text-muted-foreground"
				/>
				<Command.List className="max-h-72 overflow-y-auto p-2 text-sm">
					<Command.Empty className="px-3 py-6 text-center text-muted-foreground">
						No commands found.
					</Command.Empty>
					<Command.Group heading="Navigation">
						<Command.Item
							onSelect={() => {
								onNavigate("/");
								setOpen(false);
							}}
							className="flex cursor-pointer items-center gap-2 rounded-md px-3 py-2 aria-selected:bg-accent"
						>
							<LayoutGrid className="size-4 text-muted-foreground" />
							<span>Go to Agents Overview</span>
						</Command.Item>
						<Command.Item
							onSelect={() => {
								onNavigate("/editor");
								setOpen(false);
							}}
							className="flex cursor-pointer items-center gap-2 rounded-md px-3 py-2 aria-selected:bg-accent"
						>
							<FileCode2 className="size-4 text-muted-foreground" />
							<span>Go to Config &amp; Diff Editor</span>
						</Command.Item>
					</Command.Group>
					<Command.Group heading="Actions">
						<Command.Item
							onSelect={() => {
								onRefreshAgents();
								setOpen(false);
							}}
							className="flex cursor-pointer items-center gap-2 rounded-md px-3 py-2 aria-selected:bg-accent"
						>
							<RefreshCw className="size-4 text-muted-foreground" />
							<span>Refresh detected agents</span>
						</Command.Item>
					</Command.Group>
				</Command.List>
			</Command>
		</div>
	);
}
