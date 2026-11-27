import { useQuery } from "@tanstack/react-query";
import { Monitor, RefreshCw } from "lucide-react";
import { useRef, useState } from "react";
import {
	EmptyState,
	ErrorState,
	LoadingState,
	PageHeader,
} from "@/components/patterns";
import { Button } from "@/components/ui/button";
import { isTauriRuntime, listAgents } from "@/lib/agents";
import { AgentFilterForm, type AgentFilterValues } from "./AgentFilterForm";
import { AgentInventory } from "./AgentsTable";

const initialFilters: AgentFilterValues = { query: "", status: "all" };
export const agentsQueryKey = ["agents"] as const;

export function AgentsPage() {
	const { data, isFetching, isError, error, refetch } = useQuery({
		queryKey: agentsQueryKey,
		queryFn: listAgents,
	});
	const [filter, setFilter] = useState(initialFilters);
	const searchRef = useRef<HTMLInputElement>(null);
	const query = filter.query.trim().toLowerCase();
	const hasFilters = query !== "" || filter.status !== "all";
	const matches = data?.filter((agent) => {
		if (filter.status === "detected" && !agent.detected) return false;
		if (filter.status === "missing" && agent.detected) return false;
		return (
			!query ||
			agent.name.toLowerCase().includes(query) ||
			(agent.configPath ?? "").toLowerCase().includes(query)
		);
	});

	function refresh() {
		void refetch({ cancelRefetch: false });
	}

	return (
		<section aria-labelledby="agents-title" className="space-y-6">
			<PageHeader
				title="Local agents"
				titleId="agents-title"
				description="Find the native configuration folders for your coding agents."
				actions={
					<Button className="h-10" disabled={isFetching} onClick={refresh}>
						<RefreshCw className="size-4" aria-hidden="true" />
						{data && isFetching ? "Refreshing…" : "Refresh"}
					</Button>
				}
			/>
			{!data && !isError && (
				<LoadingState
					message="Checking local configuration folders…"
					className="min-h-44 items-start"
				/>
			)}
			{isError && (
				<ErrorState
					title={
						data
							? "Could not refresh. Showing previous results."
							: "Could not check local configuration folders."
					}
					message={error.message}
					action={
						<Button
							variant="outline"
							size="sm"
							onClick={refresh}
							disabled={isFetching}
						>
							Try again
						</Button>
					}
				/>
			)}

			{data && matches && (
				<section
					aria-labelledby="inventory-title"
					className="rounded-xl border border-border bg-card"
				>
					<div className="space-y-4 px-4 py-5 sm:px-6">
						<div className="flex flex-wrap items-baseline justify-between gap-x-4 gap-y-1">
							<h2 id="inventory-title" className="text-base font-semibold">
								Configuration folders
							</h2>
							<p role="status" className="text-xs text-muted-foreground">
								{isFetching && <span className="sr-only">Refreshing… </span>}
								<span>
									{data.filter((agent) => agent.detected).length} of{" "}
									{data.length} configurations detected
								</span>
								{hasFilters && ` · ${matches.length} matching`}
							</p>
						</div>
						<AgentFilterForm
							values={filter}
							onFilterChange={setFilter}
							searchRef={searchRef}
						/>
					</div>
					{matches.length > 0 ? (
						<AgentInventory agents={matches} />
					) : (
						<EmptyState
							className="mx-4 mb-4 border-dashed bg-background sm:mx-6 sm:mb-6"
							title={
								hasFilters
									? "No agents match your filters."
									: "No supported agents were returned."
							}
							action={
								hasFilters && (
									<Button
										variant="outline"
										size="sm"
										onClick={() => {
											setFilter(initialFilters);
											searchRef.current?.focus();
										}}
									>
										Clear filters
									</Button>
								)
							}
						/>
					)}
				</section>
			)}
			{!isTauriRuntime() && (
				<p className="flex items-center gap-2 text-xs text-muted-foreground">
					<Monitor className="size-4 shrink-0" aria-hidden="true" />
					Local configuration folders are detected in the desktop app.
				</p>
			)}
		</section>
	);
}
