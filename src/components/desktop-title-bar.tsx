import { getCurrentWindow } from "@tauri-apps/api/window";
import { Minus, Square, Terminal, X } from "lucide-react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";

const windowControls = [
	{ label: "Minimize window", method: "minimize", icon: Minus },
	{
		label: "Maximize or restore window",
		method: "toggleMaximize",
		icon: Square,
	},
	{ label: "Close window", method: "close", icon: X },
] as const;

async function applyWindowControl(
	method: (typeof windowControls)[number]["method"],
) {
	try {
		await getCurrentWindow()[method]();
	} catch {
		toast.error("Could not update the window. Try again.");
	}
}

export function DesktopTitleBar() {
	return (
		<div className="fixed inset-x-0 top-0 z-10 flex h-(--title-bar-height) select-none items-center border-b border-border bg-sidebar text-foreground">
			<div
				data-tauri-drag-region
				className="flex h-full min-w-0 flex-1 items-center gap-2 px-4"
			>
				<Terminal
					className="pointer-events-none size-4 shrink-0 text-primary"
					aria-hidden="true"
				/>
				<span className="pointer-events-none truncate text-xs font-semibold">
					Agents Manager
				</span>
			</div>
			<fieldset
				aria-label="Window controls"
				className="flex shrink-0 gap-1 px-2"
			>
				{windowControls.map(({ label, method, icon: Icon }) => (
					<Button
						key={method}
						variant="ghost"
						className={
							method === "close"
								? "size-8 p-0 hover:bg-destructive hover:text-destructive-foreground"
								: "size-8 p-0"
						}
						aria-label={label}
						title={label}
						onClick={() => {
							void applyWindowControl(method);
						}}
					>
						<Icon className="size-3.5" aria-hidden="true" />
					</Button>
				))}
			</fieldset>
		</div>
	);
}
