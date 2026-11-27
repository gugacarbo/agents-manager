import type { ComponentProps } from "react";
import { Group, Panel, Separator } from "react-resizable-panels";
import { cn } from "@/lib/utils";

export function ResizablePanelGroup({
	className,
	...props
}: ComponentProps<typeof Group>) {
	return (
		<Group
			className={cn("flex h-full w-full overflow-hidden", className)}
			{...props}
		/>
	);
}

export function ResizablePanel({
	className,
	...props
}: ComponentProps<typeof Panel>) {
	return <Panel className={cn("min-w-0", className)} {...props} />;
}

export function ResizableHandle({
	className,
	...props
}: ComponentProps<typeof Separator>) {
	return (
		<Separator
			className={cn(
				"w-1.5 shrink-0 bg-border transition-colors hover:bg-slate-600 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring",
				className,
			)}
			{...props}
		/>
	);
}
