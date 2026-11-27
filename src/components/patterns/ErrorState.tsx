import { AlertCircle } from "lucide-react";
import type { ReactNode } from "react";
import { cn } from "@/lib/utils";

interface ErrorStateProps {
	title?: string;
	message: string;
	action?: ReactNode;
	className?: string;
}

export function ErrorState({
	title = "Something went wrong",
	message,
	action,
	className,
}: ErrorStateProps) {
	return (
		<div
			role="alert"
			className={cn(
				"space-y-3 rounded-lg border border-destructive/40 bg-card p-4 text-sm text-foreground",
				className,
			)}
		>
			<div className="flex items-start gap-2.5">
				<AlertCircle
					className="mt-0.5 size-4 shrink-0 text-destructive"
					aria-hidden="true"
				/>
				<div className="space-y-1">
					<p className="font-medium text-destructive">{title}</p>
					<p className="text-muted-foreground [overflow-wrap:anywhere]">
						{message}
					</p>
				</div>
			</div>
			{action ? <div>{action}</div> : null}
		</div>
	);
}
