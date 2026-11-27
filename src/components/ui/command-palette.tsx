import { FileCode2, LayoutGrid, RefreshCw, Search } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { Button } from "@/components/ui/button";
import {
	CommandDialog,
	CommandEmpty,
	CommandGroup,
	CommandInput,
	CommandItem,
	CommandList,
	CommandSeparator,
} from "@/components/ui/command";
import { Kbd, KbdGroup } from "@/components/ui/kbd";

interface CommandPaletteProps {
	onNavigate: (path: "/" | "/demo") => void;
	showDemo: boolean;
	onRefreshAgents: () => void;
}

export function CommandPalette({
	showDemo,
	onNavigate,
	onRefreshAgents,
}: CommandPaletteProps) {
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
				setOpen((prev) => !prev);
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
				className="w-44 justify-between gap-2 bg-background/60 px-2.5 font-normal text-muted-foreground hover:text-foreground sm:w-56"
				aria-haspopup="dialog"
				aria-keyshortcuts="Control+K Meta+K"
				onClick={() => {
					trigger.current?.focus();
					setOpen(true);
				}}
			>
				<span className="flex items-center gap-2">
					<Search className="size-3.5" aria-hidden="true" />
					<span>Search…</span>
				</span>
				<KbdGroup className="hidden sm:inline-flex">
					<Kbd>Ctrl</Kbd>
					<Kbd>K</Kbd>
				</KbdGroup>
			</Button>
			<CommandDialog open={open} onOpenChange={setOpen} title="Command palette">
				<CommandInput
					ref={input}
					aria-label="Search commands"
					placeholder="Search commands…"
					onClose={() => setOpen(false)}
				/>
				<CommandList>
					<CommandEmpty>No commands found.</CommandEmpty>
					<CommandGroup heading="Navigation">
						<CommandItem
							onSelect={() => {
								setOpen(false);
								onNavigate("/");
							}}
						>
							<LayoutGrid className="size-4" aria-hidden="true" />
							Go to Agents
						</CommandItem>
						{showDemo && (
							<CommandItem
								onSelect={() => {
									setOpen(false);
									onNavigate("/demo");
								}}
							>
								<FileCode2 className="size-4" aria-hidden="true" />
								Open Diff demo
							</CommandItem>
						)}
					</CommandGroup>
					<CommandSeparator />
					<CommandGroup heading="Actions">
						<CommandItem
							onSelect={() => {
								setOpen(false);
								onRefreshAgents();
							}}
						>
							<RefreshCw className="size-4" aria-hidden="true" />
							Refresh agents
						</CommandItem>
					</CommandGroup>
				</CommandList>
			</CommandDialog>
		</>
	);
}
