import { ChevronDown, Search } from "lucide-react";
import type { Ref } from "react";

export interface AgentFilterValues {
	query: string;
	status: "all" | "detected" | "missing";
}

interface AgentFilterFormProps {
	values: AgentFilterValues;
	onFilterChange: (values: AgentFilterValues) => void;
	searchRef: Ref<HTMLInputElement>;
}

export function AgentFilterForm({
	values,
	onFilterChange,
	searchRef,
}: AgentFilterFormProps) {
	return (
		<search
			aria-label="Filter agents"
			className="flex flex-wrap items-center gap-3"
		>
			<div className="relative w-full min-w-0 sm:flex-1">
				<Search
					className="pointer-events-none absolute top-1/2 left-3 size-4 -translate-y-1/2 text-muted-foreground"
					aria-hidden="true"
				/>
				<input
					ref={searchRef}
					aria-label="Search by agent or path"
					type="search"
					value={values.query}
					onChange={(event) =>
						onFilterChange({ ...values, query: event.target.value })
					}
					placeholder="Search by agent or path"
					className="h-10 w-full rounded-md border border-input bg-background pr-3 pl-9 text-sm placeholder:text-muted-foreground"
				/>
			</div>
			<div className="relative">
				<select
					aria-label="Filter by status"
					value={values.status}
					onChange={(event) => {
						const status = event.target.value;
						if (
							status === "all" ||
							status === "detected" ||
							status === "missing"
						) {
							onFilterChange({ ...values, status });
						}
					}}
					className="h-10 cursor-pointer appearance-none rounded-md border border-input bg-background pr-9 pl-3 text-sm"
				>
					<option value="all">All statuses</option>
					<option value="detected">Detected</option>
					<option value="missing">Not detected</option>
				</select>
				<ChevronDown
					className="pointer-events-none absolute top-1/2 right-3 size-4 -translate-y-1/2 text-muted-foreground"
					aria-hidden="true"
				/>
			</div>
		</search>
	);
}
