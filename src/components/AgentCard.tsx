import { FolderCheck, FolderX } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import type { AgentSummary } from "@/lib/schemas";

export function AgentCard({ agent }: { agent: AgentSummary }) {
	const statusLabel = agent.detected ? "Detected" : "Not detected";

	return (
		<Card className="flex flex-col justify-between">
			<CardHeader className="flex-row items-start justify-between gap-3 space-y-0">
				<div>
					<CardTitle>{agent.name}</CardTitle>
					<p className="mt-1 font-mono text-xs text-muted-foreground">
						{agent.kind}
					</p>
				</div>
				<Badge
					role="status"
					variant={agent.detected ? "success" : "muted"}
					className="gap-1"
				>
					{agent.detected ? (
						<FolderCheck className="size-3.5" aria-hidden="true" />
					) : (
						<FolderX className="size-3.5" aria-hidden="true" />
					)}
					<span>{statusLabel}</span>
				</Badge>
			</CardHeader>
			<CardContent>
				<p className="truncate font-mono text-xs text-muted-foreground">
					{agent.configPath ?? "No configuration path detected"}
				</p>
			</CardContent>
		</Card>
	);
}
