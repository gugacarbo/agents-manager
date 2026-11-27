import { cva, type VariantProps } from "class-variance-authority";
import type { HTMLAttributes } from "react";
import { cn } from "@/lib/utils";

const badgeVariants = cva(
	"inline-flex items-center rounded-full px-2.5 py-1 text-xs font-medium ring-1 ring-inset",
	{
		variants: {
			variant: {
				default: "bg-secondary text-secondary-foreground ring-border",
				success: "bg-emerald-500/15 text-emerald-300 ring-emerald-500/30",
				muted: "bg-slate-800 text-slate-300 ring-slate-700",
			},
		},
		defaultVariants: {
			variant: "default",
		},
	},
);

type BadgeProps = HTMLAttributes<HTMLSpanElement> &
	VariantProps<typeof badgeVariants>;

export function Badge({ className, variant, ...props }: BadgeProps) {
	return (
		<span className={cn(badgeVariants({ variant }), className)} {...props} />
	);
}
