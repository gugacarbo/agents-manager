import { Command } from "cmdk";
import { FileCode2, LayoutGrid, RefreshCw, Search, X } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { Button } from "@/components/ui/button";

interface CommandPaletteProps {
	onNavigate: (path: "/" | "/editor") => void;
	onRefreshAgents: () => void;
}

export function CommandPalette({
	onNavigate,
	onRefreshAgents,
}: CommandPaletteProps) {
	const dialog = useRef<HTMLDialogElement>(null);
	const trigger = useRef<HTMLButtonElement>(null);
	const input = useRef<HTMLInputElement>(null);
	const [open, setOpen] = useState(false);

	useEffect(() => {
		if (open) input.current?.focus();
	}, [open]);

	useEffect(() => {
		function handleKeyDown(event: KeyboardEvent) {
			if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "k") {
				event.preventDefault();
				if (event.repeat) return;
				if (dialog.current?.open) {
					dialog.current.close();
				} else {
					dialog.current?.showModal();
					setOpen(true);
				}
			}
		}
		window.addEventListener("keydown", handleKeyDown);
		return () => window.removeEventListener("keydown", handleKeyDown);
	}, []);

	return (
		<>
			<Button
				ref={trigger}
				type="button"
				variant="outline"
				size="sm"
				aria-haspopup="dialog"
				aria-keyshortcuts="Control+K Meta+K"
				onClick={() => {
					trigger.current?.focus();
					dialog.current?.showModal();
					setOpen(true);
				}}
			>
				<Search className="size-4" aria-hidden="true" />
				Search
				<kbd className="ml-2 hidden rounded border border-border bg-background px-1.5 font-mono text-xs text-muted-foreground sm:inline">
					Ctrl+K
				</kbd>
			</Button>
			<dialog
				ref={dialog}
				aria-label="Command palette"
				className="fixed inset-x-0 top-[min(12vh,6rem)] mx-auto w-[min(32rem,calc(100%-2rem))] rounded-xl border border-border bg-popover p-0 text-popover-foreground shadow-2xl backdrop:bg-slate-950/75"
				onClose={() => {
					if (!dialog.current?.open) setOpen(false);
				}}
			>
				{open && (
					<Command className="text-sm">
						<div className="flex items-center gap-2 border-b border-border p-2">
							<Command.Input
								ref={input}
								aria-label="Search commands"
								placeholder="Search commands…"
								className="min-w-0 flex-1 rounded-md bg-transparent px-3 py-2 placeholder:text-muted-foreground"
							/>
							<Button
								variant="ghost"
								size="sm"
								aria-label="Close command palette"
								onClick={() => dialog.current?.close()}
							>
								<X className="size-4" aria-hidden="true" />
							</Button>
						</div>
						<Command.List className="max-h-[min(60vh,20rem)] overflow-y-auto p-2">
							<Command.Empty className="px-3 py-5 text-muted-foreground">
								No commands found.
							</Command.Empty>
							<Command.Item
								className={itemStyle}
								onSelect={() => {
									dialog.current?.close();
									onNavigate("/");
								}}
							>
								<LayoutGrid className="size-4" aria-hidden="true" />
								Go to Agents
							</Command.Item>
							<Command.Item
								className={itemStyle}
								onSelect={() => {
									dialog.current?.close();
									onNavigate("/editor");
								}}
							>
								<FileCode2 className="size-4" aria-hidden="true" />
								Open Diff demo
							</Command.Item>
							<Command.Item
								className={itemStyle}
								onSelect={() => {
									dialog.current?.close();
									onRefreshAgents();
								}}
							>
								<RefreshCw className="size-4" aria-hidden="true" />
								Refresh agents
							</Command.Item>
						</Command.List>
					</Command>
				)}
			</dialog>
		</>
	);
}

const itemStyle =
	"flex cursor-pointer items-center gap-2 rounded-md px-3 py-2 aria-selected:bg-selection aria-selected:text-selection-foreground";
