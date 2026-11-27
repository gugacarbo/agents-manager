import { useQuery } from "@tanstack/react-query";
import { listAgents } from "./lib/agents";
import type { AgentSummary } from "./lib/schemas";

function AgentCard({ agent }: { agent: AgentSummary }) {
	const statusLabel = agent.detected ? "Detected" : "Not detected";
	const badgeClass = agent.detected
		? "bg-emerald-500/15 text-emerald-300 ring-emerald-500/30"
		: "bg-slate-800 text-slate-300 ring-slate-700";

	return (
		<article className="flex flex-col justify-between gap-4 rounded-xl border border-slate-800 bg-slate-900/70 p-5 shadow-sm">
			<div className="flex items-start justify-between gap-3">
				<div>
					<h2 className="text-lg font-semibold text-slate-100">{agent.name}</h2>
					<p className="mt-1 font-mono text-xs text-slate-400">{agent.kind}</p>
				</div>
				<span
					role="status"
					className={`inline-flex items-center rounded-full px-2.5 py-1 text-xs font-medium ring-1 ring-inset ${badgeClass}`}
				>
					{statusLabel}
				</span>
			</div>
			<p className="text-xs text-slate-400">
				{agent.configPath ?? "No configuration path detected"}
			</p>
		</article>
	);
}

export function App() {
	const { data, isLoading, isError, error } = useQuery({
		queryKey: ["agents"],
		queryFn: listAgents,
	});

	return (
		<main className="min-h-screen bg-slate-950 px-6 py-12 text-slate-100">
			<div className="mx-auto max-w-5xl space-y-8">
				<header className="space-y-2 border-b border-slate-800 pb-6">
					<h1 className="text-3xl font-bold tracking-tight text-white">
						Agents Manager
					</h1>
					<p className="text-sm text-slate-400">
						Inspect detected coding agents and manage local configurations from
						a unified workspace.
					</p>
				</header>

				{isLoading ? (
					<p role="status" className="text-sm text-slate-400">
						Loading agents...
					</p>
				) : isError ? (
					<p
						role="alert"
						className="rounded-lg border border-rose-500/30 bg-rose-500/10 p-4 text-sm text-rose-200"
					>
						Failed to load agents:{" "}
						{error instanceof Error ? error.message : "Unknown error"}
					</p>
				) : (
					<section
						aria-label="Supported agents"
						className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3"
					>
						{(data ?? []).map((agent) => (
							<AgentCard key={agent.kind} agent={agent} />
						))}
					</section>
				)}
			</div>
		</main>
	);
}
