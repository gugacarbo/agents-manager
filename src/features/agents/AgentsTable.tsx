import { Circle, CircleCheck } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import type { AgentSummary } from "@/lib/schemas";

export function AgentInventory({ agents }: { agents: AgentSummary[] }) {
	return (
		<div className="overflow-hidden rounded-lg border border-border bg-card">
			<div
				aria-hidden="true"
				className="hidden agent-inventory-grid gap-4 border-b border-border bg-background/40 px-4 py-2.5 text-xs font-medium text-muted-foreground lg:grid"
			>
				<span>Agent</span>
				<span>Detection</span>
				<span>Configuration folder</span>
			</div>
			<ul aria-label="Agent inventory" className="divide-y divide-border">
				{agents.map((agent) => {
					const Icon = agent.detected ? CircleCheck : Circle;
					return (
						<li
							key={agent.kind}
							className="agent-inventory-grid grid items-start gap-x-4 gap-y-2 px-4 py-3 text-sm lg:items-center"
						>
							<span className="font-medium wrap-anywhere">{agent.name}</span>
							<span>
								<Badge
									variant={agent.detected ? "success" : "muted"}
									className="gap-1.5 whitespace-nowrap"
								>
									<Icon className="size-3" aria-hidden="true" />
									{agent.detected ? "Detected" : "Not detected"}
								</Badge>
							</span>
							<div className="col-span-2 min-w-0 text-xs text-muted-foreground lg:col-span-1">
								{agent.configPath ? (
									<code className="font-mono select-text wrap-anywhere">
										{agent.configPath}
									</code>
								) : (
									"Configuration folder unavailable"
								)}
							</div>
						</li>
					);
				})}
			</ul>
		</div>
	);
}
