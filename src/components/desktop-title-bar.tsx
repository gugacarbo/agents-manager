import { getCurrentWindow } from "@tauri-apps/api/window";
import { Copy, Minus, Square, X } from "lucide-react";
import {
	type ReactNode,
	useCallback,
	useEffect,
	useRef,
	useState,
} from "react";
import { toast } from "sonner";
import { BrandMark } from "@/components/brand-mark";
import { Button } from "@/components/ui/button";

interface DesktopTitleBarProps {
	leading?: ReactNode;
	breadcrumb?: ReactNode;
	actions?: ReactNode;
	offsetSidebar?: boolean;
}

interface WindowFrameState {
	maximized: boolean;
	fullscreen: boolean;
}

export function DesktopTitleBar({
	leading,
	breadcrumb,
	actions,
	offsetSidebar = false,
}: DesktopTitleBarProps) {
	const [frameState, setFrameState] = useState<WindowFrameState>({
		maximized: false,
		fullscreen: false,
	});
	const revisionRef = useRef(0);

	const syncWindowState = useCallback(async () => {
		const current = ++revisionRef.current;
		try {
			const appWindow = getCurrentWindow();
			const [maximized, fullscreen] = await Promise.all([
				appWindow.isMaximized(),
				appWindow.isFullscreen(),
			]);
			if (current === revisionRef.current) {
				setFrameState({
					maximized: maximized === true,
					fullscreen: fullscreen === true,
				});
			}
		} catch {
			if (current === revisionRef.current) {
				setFrameState({ maximized: false, fullscreen: false });
			}
		}
	}, []);

	useEffect(() => {
		const update = () => {
			void syncWindowState();
		};
		window.addEventListener("resize", update);
		window.addEventListener("focus", update);
		update();

		return () => {
			++revisionRef.current;
			window.removeEventListener("resize", update);
			window.removeEventListener("focus", update);
		};
	}, [syncWindowState]);

	const isExpanded = frameState.maximized || frameState.fullscreen;
	const expandLabel = isExpanded ? "Restore window" : "Maximize window";

	const handleWindowControl = async (
		action: "minimize" | "expand" | "close",
	) => {
		try {
			const appWindow = getCurrentWindow();
			if (action === "minimize") {
				await appWindow.minimize();
			} else if (action === "close") {
				await appWindow.close();
			} else if (frameState.fullscreen) {
				await appWindow.setFullscreen(false);
				await syncWindowState();
			} else {
				await appWindow.toggleMaximize();
				await syncWindowState();
			}
		} catch {
			toast.error("Could not update the window. Try again.");
		}
	};

	return (
		<div
			data-testid="desktop-title-bar"
			className="relative z-20 flex min-h-(--title-bar-height) shrink-0 select-none items-center border-b border-border bg-sidebar py-1.5 text-foreground"
		>
			{leading ? (
				<div className="flex shrink-0 items-center gap-1.5 pl-2">
					{leading}
					{breadcrumb ? (
						<div className="h-4 w-px bg-border" aria-hidden="true" />
					) : null}
				</div>
			) : null}
			<div
				data-tauri-drag-region
				className="flex h-full min-w-0 flex-1 items-center gap-2 px-3"
			>
				{breadcrumb ? (
					<div className="pointer-events-none flex min-w-0 items-center gap-1.5 truncate text-xs text-muted-foreground">
						{breadcrumb}
					</div>
				) : (
					<>
						<BrandMark
							variant="glyph"
							className={
								offsetSidebar
									? "pointer-events-none size-4 shrink-0 text-primary md:hidden"
									: "pointer-events-none size-4 shrink-0 text-primary"
							}
						/>
						<span
							className={
								offsetSidebar
									? "pointer-events-none truncate text-xs font-semibold md:hidden"
									: "pointer-events-none truncate text-xs font-semibold"
							}
						>
							Agents Manager
						</span>
					</>
				)}
			</div>
			{actions ? (
				<div className="flex shrink-0 items-center gap-1.5 px-2">
					{actions}
					<div className="ml-1 h-4 w-px bg-border" aria-hidden="true" />
				</div>
			) : null}
			<fieldset
				aria-label="Window controls"
				className="flex shrink-0 gap-1 px-2"
			>
				<Button
					variant="ghost"
					className="size-8 p-0"
					aria-label="Minimize window"
					title="Minimize window"
					onClick={() => {
						void handleWindowControl("minimize");
					}}
				>
					<Minus className="size-3.5" aria-hidden="true" />
				</Button>
				<Button
					variant="ghost"
					className="size-8 p-0"
					aria-label={expandLabel}
					title={expandLabel}
					onClick={() => {
						void handleWindowControl("expand");
					}}
				>
					{isExpanded ? (
						<Copy className="size-3.5 -scale-x-100" aria-hidden="true" />
					) : (
						<Square className="size-3.5" aria-hidden="true" />
					)}
				</Button>
				<Button
					variant="ghost"
					className="size-8 p-0 hover:bg-destructive hover:text-destructive-foreground"
					aria-label="Close window"
					title="Close window"
					onClick={() => {
						void handleWindowControl("close");
					}}
				>
					<X className="size-3.5" aria-hidden="true" />
				</Button>
			</fieldset>
		</div>
	);
}
