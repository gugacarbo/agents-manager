import { Command as CommandPrimitive } from "cmdk";
import { Search, X } from "lucide-react";
import { type ComponentProps, type ReactNode, useEffect, useRef } from "react";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

function Command({
	className,
	...props
}: ComponentProps<typeof CommandPrimitive>) {
	return (
		<CommandPrimitive
			data-slot="command"
			className={cn(
				"flex w-full flex-col overflow-hidden rounded-xl bg-popover text-popover-foreground",
				className,
			)}
			{...props}
		/>
	);
}

interface CommandDialogProps {
	open: boolean;
	onOpenChange: (open: boolean) => void;
	title?: string;
	children: ReactNode;
}

export function CommandDialog({
	open,
	onOpenChange,
	title = "Command palette",
	children,
}: CommandDialogProps) {
	const dialogRef = useRef<HTMLDialogElement>(null);

	useEffect(() => {
		const dialog = dialogRef.current;
		if (!dialog) return;
		if (open && !dialog.open) {
			dialog.showModal();
		} else if (!open && dialog.open) {
			dialog.close();
		}
	}, [open]);

	return (
		<dialog
			ref={dialogRef}
			data-slot="command-dialog"
			aria-label={title}
			className="fixed inset-x-0 top-[min(12vh,6rem)] mx-auto w-[min(32rem,calc(100%-2rem))] rounded-xl border border-border bg-popover p-0 text-popover-foreground shadow-2xl backdrop:bg-black/65"
			onClose={() => {
				onOpenChange(false);
			}}
			onClick={(event) => {
				if (event.target === dialogRef.current) {
					onOpenChange(false);
				}
			}}
			onKeyDown={(event) => {
				if (event.key === "Escape") {
					onOpenChange(false);
				}
			}}
		>
			{open ? <Command>{children}</Command> : null}
		</dialog>
	);
}

interface CommandInputProps
	extends ComponentProps<typeof CommandPrimitive.Input> {
	onClose?: () => void;
}

export function CommandInput({
	className,
	onClose,
	...props
}: CommandInputProps) {
	return (
		<div
			data-slot="command-input-wrapper"
			className="flex h-12 items-center gap-2.5 border-b border-border px-3.5"
		>
			<Search
				className="size-4 shrink-0 text-muted-foreground"
				aria-hidden="true"
			/>
			<CommandPrimitive.Input
				data-slot="command-input"
				className={cn(
					"flex h-full w-full bg-transparent text-sm text-foreground outline-none focus-visible:outline-none placeholder:text-muted-foreground disabled:cursor-not-allowed disabled:opacity-50",
					className,
				)}
				{...props}
			/>
			{onClose ? (
				<Button
					variant="ghost"
					size="sm"
					className="size-7 shrink-0 p-0 text-muted-foreground hover:text-foreground"
					aria-label="Close command palette"
					onClick={onClose}
				>
					<X className="size-4" aria-hidden="true" />
				</Button>
			) : null}
		</div>
	);
}

export function CommandList({
	className,
	...props
}: ComponentProps<typeof CommandPrimitive.List>) {
	return (
		<CommandPrimitive.List
			data-slot="command-list"
			className={cn(
				"max-h-[min(60vh,20rem)] scroll-py-1 overflow-x-hidden overflow-y-auto p-1.5",
				className,
			)}
			{...props}
		/>
	);
}

export function CommandEmpty({
	className,
	...props
}: ComponentProps<typeof CommandPrimitive.Empty>) {
	return (
		<CommandPrimitive.Empty
			data-slot="command-empty"
			className={cn(
				"py-6 text-center text-sm text-muted-foreground",
				className,
			)}
			{...props}
		/>
	);
}

export function CommandGroup({
	className,
	...props
}: ComponentProps<typeof CommandPrimitive.Group>) {
	return (
		<CommandPrimitive.Group
			data-slot="command-group"
			className={cn(
				"overflow-hidden p-1 text-foreground [&_[cmdk-group-heading]]:px-2 [&_[cmdk-group-heading]]:py-1.5 [&_[cmdk-group-heading]]:text-xs [&_[cmdk-group-heading]]:font-medium [&_[cmdk-group-heading]]:text-muted-foreground",
				className,
			)}
			{...props}
		/>
	);
}

export function CommandSeparator({
	className,
	...props
}: ComponentProps<typeof CommandPrimitive.Separator>) {
	return (
		<CommandPrimitive.Separator
			data-slot="command-separator"
			className={cn("-mx-1 my-1 h-px bg-border", className)}
			{...props}
		/>
	);
}

export function CommandItem({
	className,
	...props
}: ComponentProps<typeof CommandPrimitive.Item>) {
	return (
		<CommandPrimitive.Item
			data-slot="command-item"
			className={cn(
				"relative flex cursor-pointer select-none items-center gap-2.5 rounded-md px-2.5 py-2 text-sm outline-none focus-visible:outline-none data-[disabled=true]:pointer-events-none data-[selected=true]:bg-selection data-[selected=true]:text-selection-foreground data-[disabled=true]:opacity-50 [&_svg]:pointer-events-none [&_svg]:shrink-0",
				className,
			)}
			{...props}
		/>
	);
}
