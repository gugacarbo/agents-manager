import { Link } from "@tanstack/react-router";
import {
	ArrowUpRight,
	Asterisk,
	Circle,
	CircleCheck,
	Orbit,
	SquareTerminal,
} from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { isTauriRuntime } from "@/lib/agents";
import type { AgentSummary } from "@/lib/schemas";

const agentIcons = {
	"claude-code": Asterisk,
	codex: SquareTerminal,
	antigravity: Orbit,
};

export function AgentInventory({ agents }: { agents: AgentSummary[] }) {
	const desktop = isTauriRuntime();

	return (
		<div className="border-t border-border">
			<div
				aria-hidden="true"
				className="agent-inventory-grid hidden gap-4 border-b border-border bg-muted/55 px-6 py-2.5 text-xs font-medium text-muted-foreground lg:grid"
			>
				<span>Agent</span>
				<span>Configuration folder</span>
				<span>Detection</span>
			</div>
			<ul aria-label="Agent inventory" className="divide-y divide-border">
				{agents.map((agent) => {
					const AgentIcon = agentIcons[agent.kind];
					const StatusIcon = agent.detected ? CircleCheck : Circle;

					return (
						<li
							key={agent.kind}
							className="agent-inventory-grid grid items-center gap-x-4 gap-y-2 px-4 py-4 text-sm sm:px-6"
						>
							<div className="flex min-w-0 items-center gap-3">
								<span className="grid size-9 shrink-0 place-items-center rounded-md border border-border bg-muted text-foreground">
									<AgentIcon className="size-4" aria-hidden="true" />
								</span>
								<div className="min-w-0 space-y-1">
									{agent.detected ? (
										<Link
											to="/agents/$agentKind"
											params={{ agentKind: agent.kind }}
											className="inline-flex min-h-7 items-center gap-1.5 rounded-sm font-semibold wrap-anywhere hover:text-selection-foreground hover:underline hover:underline-offset-4"
										>
											{agent.name}
											<ArrowUpRight
												className="size-3.5 shrink-0 text-muted-foreground"
												aria-hidden="true"
											/>
										</Link>
									) : (
										<span className="font-medium wrap-anywhere">
											{agent.name}
										</span>
									)}
									{agent.kind === "claude-code" && desktop && (
										<Link
											to="/rules"
											className="block w-fit rounded-sm text-xs text-muted-foreground underline underline-offset-4 hover:text-foreground"
										>
											Edit rules
										</Link>
									)}
								</div>
							</div>
							<div className="order-last col-span-2 min-w-0 text-xs text-muted-foreground lg:order-0 lg:col-span-1">
								{agent.configPath ? (
									<code className="font-mono leading-relaxed select-text wrap-anywhere">
										{agent.configPath}
									</code>
								) : (
									"Configuration folder unavailable"
								)}
							</div>
							<span>
								<Badge
									variant={agent.detected ? "success" : "muted"}
									className="gap-1.5 whitespace-nowrap"
								>
									<StatusIcon className="size-3" aria-hidden="true" />
									{agent.detected ? "Detected" : "Not detected"}
								</Badge>
							</span>
						</li>
					);
				})}
			</ul>
		</div>
	);
}
