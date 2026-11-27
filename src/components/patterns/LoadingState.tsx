import { Loader2 } from "lucide-react";
import { cn } from "@/lib/utils";

interface LoadingStateProps {
	message?: string;
	className?: string;
}

export function LoadingState({
	message = "Loading...",
	className,
}: LoadingStateProps) {
	return (
		<div
			role="status"
			className={cn(
				"flex items-center gap-2 rounded-lg border border-border bg-card p-4 text-sm text-muted-foreground",
				className,
			)}
		>
			<Loader2 className="size-4 animate-spin" aria-hidden="true" />
			<span>{message}</span>
		</div>
	);
}
