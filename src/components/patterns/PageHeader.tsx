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
	headingLevel = "h1",
	className,
}: PageHeaderProps) {
	const Heading = headingLevel;

	return (
		<div
			className={cn(
				"flex flex-wrap items-end justify-between gap-4 border-b border-border pb-6",
				className,
			)}
		>
			<div className="min-w-0 space-y-2">
				<Heading
					id={titleId}
					className="text-3xl leading-tight font-semibold tracking-tight text-foreground wrap-anywhere sm:text-4xl"
				>
					{title}
				</Heading>
				{description ? (
					<p className="max-w-2xl text-sm leading-relaxed text-muted-foreground">
						{description}
					</p>
				) : null}
			</div>
			{actions ? (
				<div className="flex flex-wrap items-center gap-2">{actions}</div>
			) : null}
		</div>
	);
}
