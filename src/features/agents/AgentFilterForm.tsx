import { useForm } from "@tanstack/react-form";
import { Search, X } from "lucide-react";
import { z } from "zod";
import { Button } from "@/components/ui/button";

const agentFilterSchema = z.object({
	query: z.string().max(100),
	status: z.enum(["all", "detected", "missing"]),
});

export type AgentFilterValues = z.infer<typeof agentFilterSchema>;

interface AgentFilterFormProps {
	initialValues: AgentFilterValues;
	onFilterChange: (values: AgentFilterValues) => void;
}

export function AgentFilterForm({
	initialValues,
	onFilterChange,
}: AgentFilterFormProps) {
	const form = useForm({
		defaultValues: initialValues,
		validators: {
			onChange: agentFilterSchema,
		},
		onSubmit: ({ value }) => {
			onFilterChange(value);
		},
	});

	return (
		<form
			aria-label="Filter agents"
			onSubmit={(event) => {
				event.preventDefault();
				void form.handleSubmit();
			}}
			className="flex flex-wrap items-center gap-3"
		>
			<form.Field name="query">
				{(field) => (
					<div className="relative min-w-56 flex-1">
						<Search
							className="pointer-events-none absolute top-1/2 left-3 size-4 -translate-y-1/2 text-muted-foreground"
							aria-hidden="true"
						/>
						<input
							aria-label="Search agents"
							type="text"
							value={field.state.value}
							onChange={(event) => {
								const nextQuery = event.target.value;
								field.handleChange(nextQuery);
								onFilterChange({
									query: nextQuery,
									status: form.state.values.status,
								});
							}}
							placeholder="Filter by agent name or path..."
							className="h-9 w-full rounded-md border border-border bg-card pr-3 pl-9 text-sm text-foreground outline-none placeholder:text-muted-foreground focus:ring-2 focus:ring-ring"
						/>
					</div>
				)}
			</form.Field>

			<form.Field name="status">
				{(field) => (
					<select
						aria-label="Filter by status"
						value={field.state.value}
						onChange={(event) => {
							const nextStatus = event.target
								.value as AgentFilterValues["status"];
							field.handleChange(nextStatus);
							onFilterChange({
								query: form.state.values.query,
								status: nextStatus,
							});
						}}
						className="h-9 rounded-md border border-border bg-card px-3 text-sm text-foreground outline-none focus:ring-2 focus:ring-ring"
					>
						<option value="all">All statuses</option>
						<option value="detected">Detected only</option>
						<option value="missing">Not detected</option>
					</select>
				)}
			</form.Field>

			<Button
				variant="outline"
				size="sm"
				onClick={() => {
					const resetValues: AgentFilterValues = {
						query: "",
						status: "all",
					};
					form.reset(resetValues);
					onFilterChange(resetValues);
				}}
			>
				<X className="size-3.5" aria-hidden="true" />
				<span>Reset</span>
			</Button>
		</form>
	);
}
