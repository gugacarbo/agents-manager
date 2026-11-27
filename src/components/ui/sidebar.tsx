import { PanelLeft } from "lucide-react";
import {
	type ComponentProps,
	createContext,
	type ReactNode,
	useCallback,
	useContext,
	useEffect,
	useMemo,
	useState,
} from "react";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

const SIDEBAR_STORAGE_KEY = "agents-manager-sidebar-open";

interface SidebarContextValue {
	open: boolean;
	state: "expanded" | "collapsed";
	toggleSidebar: () => void;
}

const SidebarContext = createContext<SidebarContextValue | null>(null);

function useSidebar() {
	const context = useContext(SidebarContext);
	if (!context) {
		throw new Error("useSidebar must be used within a SidebarProvider.");
	}
	return context;
}

interface SidebarProviderProps extends ComponentProps<"div"> {
	defaultOpen?: boolean;
}

export function SidebarProvider({
	defaultOpen = true,
	className,
	children,
	...props
}: SidebarProviderProps) {
	const [open, setOpen] = useState(() => {
		if (typeof window === "undefined") return defaultOpen;
		try {
			const stored = window.localStorage.getItem(SIDEBAR_STORAGE_KEY);
			if (stored === "true") return true;
			if (stored === "false") return false;
		} catch {
			// Ignore unavailable storage.
		}
		return defaultOpen;
	});

	const toggleSidebar = useCallback(() => {
		setOpen((prev) => {
			const next = !prev;
			try {
				window.localStorage.setItem(SIDEBAR_STORAGE_KEY, String(next));
			} catch {
				// Ignore unavailable storage.
			}
			return next;
		});
	}, []);

	useEffect(() => {
		const handleKeyDown = (event: KeyboardEvent) => {
			if (
				(event.ctrlKey || event.metaKey) &&
				event.key.toLowerCase() === "b" &&
				!event.repeat
			) {
				event.preventDefault();
				toggleSidebar();
			}
		};
		window.addEventListener("keydown", handleKeyDown);
		return () => window.removeEventListener("keydown", handleKeyDown);
	}, [toggleSidebar]);

	const state: SidebarContextValue["state"] = open ? "expanded" : "collapsed";
	const value = useMemo(
		() => ({ open, state, toggleSidebar }),
		[open, state, toggleSidebar],
	);

	return (
		<SidebarContext.Provider value={value}>
			<div
				data-slot="sidebar-wrapper"
				data-state={state}
				className={cn(
					"group/sidebar-wrapper flex min-h-screen w-full flex-col md:h-screen md:flex-row md:overflow-hidden",
					className,
				)}
				{...props}
			>
				{children}
			</div>
		</SidebarContext.Provider>
	);
}

export function Sidebar({
	className,
	children,
	...props
}: ComponentProps<"aside">) {
	const { state } = useSidebar();

	return (
		<aside
			data-slot="sidebar"
			data-state={state}
			className={cn(
				"group/sidebar flex w-full shrink-0 flex-col overflow-x-hidden border-b border-border bg-sidebar text-foreground transition-[width] duration-200 ease-linear md:h-full md:overflow-hidden md:border-r md:border-b-0",
				state === "expanded" ? "md:w-54" : "md:w-13",
				className,
			)}
			{...props}
		>
			{children}
		</aside>
	);
}

export function SidebarHeader({ className, ...props }: ComponentProps<"div">) {
	return (
		<div
			data-slot="sidebar-header"
			className={cn(
				"flex min-h-12 shrink-0 items-center gap-3 px-2.5 py-1.5",
				className,
			)}
			{...props}
		/>
	);
}

export function SidebarContent({ className, ...props }: ComponentProps<"nav">) {
	return (
		<nav
			data-slot="sidebar-content"
			className={cn(
				"flex min-h-0 flex-1 flex-wrap gap-1 overflow-x-hidden px-2 pb-3 md:flex-col md:flex-nowrap md:overflow-y-auto md:pt-2",
				className,
			)}
			{...props}
		/>
	);
}

export function SidebarGroup({ className, ...props }: ComponentProps<"div">) {
	return (
		<div
			data-slot="sidebar-group"
			className={cn(
				"flex min-w-0 flex-wrap gap-1 md:flex-col md:flex-nowrap",
				className,
			)}
			{...props}
		/>
	);
}

export function SidebarGroupLabel({
	className,
	...props
}: ComponentProps<"p">) {
	return (
		<p
			data-slot="sidebar-group-label"
			className={cn(
				"mb-1.5 hidden px-2.5 text-xs font-medium text-muted-foreground group-data-[state=collapsed]/sidebar:md:hidden md:block",
				className,
			)}
			{...props}
		/>
	);
}

export function SidebarMenu({ className, ...props }: ComponentProps<"div">) {
	return (
		<div
			data-slot="sidebar-menu"
			className={cn(
				"flex min-w-0 flex-wrap gap-1 md:flex-col md:flex-nowrap",
				className,
			)}
			{...props}
		/>
	);
}

export function SidebarMenuItem({
	className,
	...props
}: ComponentProps<"div">) {
	return (
		<div
			data-slot="sidebar-menu-item"
			className={cn("min-w-0", className)}
			{...props}
		/>
	);
}

interface SidebarMenuButtonProps {
	className?: string;
	children: ReactNode;
}

export function SidebarMenuLabel({
	className,
	children,
}: SidebarMenuButtonProps) {
	return (
		<span
			className={cn(
				"min-w-0 truncate group-data-[state=collapsed]/sidebar:md:hidden",
				className,
			)}
		>
			{children}
		</span>
	);
}

export function SidebarFooter({ className, ...props }: ComponentProps<"div">) {
	return (
		<div
			data-slot="sidebar-footer"
			className={cn(
				"mt-auto hidden shrink-0 items-center gap-3 border-t border-border px-4.5 py-4 md:flex",
				className,
			)}
			{...props}
		/>
	);
}

export function SidebarInset({ className, ...props }: ComponentProps<"div">) {
	return (
		<div
			data-slot="sidebar-inset"
			className={cn(
				"flex min-h-0 min-w-0 flex-1 flex-col bg-background md:h-full md:overflow-hidden",
				className,
			)}
			{...props}
		/>
	);
}

export function SidebarTrigger({
	className,
	...props
}: ComponentProps<typeof Button>) {
	const { open, toggleSidebar } = useSidebar();
	const label = open ? "Collapse sidebar" : "Expand sidebar";

	return (
		<Button
			data-slot="sidebar-trigger"
			variant="ghost"
			size="sm"
			className={cn("hidden size-8 p-0 md:inline-flex", className)}
			aria-label={label}
			aria-expanded={open}
			title={`${label} (Ctrl+B)`}
			onClick={toggleSidebar}
			{...props}
		>
			<PanelLeft className="size-4" aria-hidden="true" />
		</Button>
	);
}

if (import.meta.hot) {
	import.meta.hot.accept(() => {
		window.location.reload();
	});
}
