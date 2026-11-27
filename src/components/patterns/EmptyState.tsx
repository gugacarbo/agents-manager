import type { ReactNode } from "react";
import { cn } from "@/lib/utils";

interface EmptyStateProps {
	title: string;
	description?: string;
	action?: ReactNode;
	className?: string;
}

export function EmptyState({
	title,
	description,
	action,
	className,
}: EmptyStateProps) {
	return (
		<div
			className={cn(
				"space-y-3 rounded-lg border border-border bg-card p-6 text-sm",
				className,
			)}
		>
			<div className="space-y-1">
				<p className="font-medium text-foreground">{title}</p>
				{description ? (
					<p className="text-muted-foreground">{description}</p>
				) : null}
			</div>
			{action ? <div>{action}</div> : null}
		</div>
	);
}
