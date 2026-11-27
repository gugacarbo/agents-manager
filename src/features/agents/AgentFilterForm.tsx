import { Search } from "lucide-react";
import type { Ref } from "react";
import { Input } from "@/components/ui/input";
import {
	NativeSelect,
	NativeSelectOption,
} from "@/components/ui/native-select";

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
				<Input
					ref={searchRef}
					aria-label="Search by agent or path"
					type="search"
					value={values.query}
					onChange={(event) =>
						onFilterChange({ ...values, query: event.target.value })
					}
					placeholder="Search by agent or path"
					className="h-10 pl-9"
				/>
			</div>
			<NativeSelect
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
				className="h-10 cursor-pointer"
			>
				<NativeSelectOption value="all">All statuses</NativeSelectOption>
				<NativeSelectOption value="detected">Detected</NativeSelectOption>
				<NativeSelectOption value="missing">Not detected</NativeSelectOption>
			</NativeSelect>
		</search>
	);
}
