import type { ReactNode } from "react";
import { cn } from "@/lib/utils";

interface PageHeaderProps {
	title: string;
	description?: string;
	actions?: ReactNode;
	titleId?: string;
	headingLevel?: "h1" | "h2";
	className?: string;
}

export function PageHeader({
	title,
	description,
	actions,
	titleId,
	headingLevel = "h2",
	className,
}: PageHeaderProps) {
	const Heading = headingLevel;

	return (
		<div
			className={cn(
				"flex flex-wrap items-start justify-between gap-4",
				className,
			)}
		>
			<div className="space-y-1">
				<Heading
					id={titleId}
					className="text-2xl font-semibold tracking-tight text-foreground"
				>
					{title}
				</Heading>
				{description ? (
					<p className="text-sm text-muted-foreground">{description}</p>
				) : null}
			</div>
			{actions ? (
				<div className="flex items-center gap-2">{actions}</div>
			) : null}
		</div>
	);
}
