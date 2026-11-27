import {
	createColumnHelper,
	tableFeatures,
	useTable,
} from "@tanstack/react-table";
import { Badge } from "@/components/ui/badge";
import type { AgentSummary } from "@/lib/schemas";

const features = tableFeatures({});
const columnHelper = createColumnHelper<typeof features, AgentSummary>();

const columns = columnHelper.columns([
	columnHelper.accessor("name", {
		header: "Agent",
		cell: (info) => (
			<span className="font-medium text-foreground">{info.getValue()}</span>
		),
	}),
	columnHelper.accessor("kind", {
		header: "Adapter Key",
		cell: (info) => (
			<code className="rounded bg-secondary px-1.5 py-0.5 font-mono text-xs text-muted-foreground">
				{info.getValue()}
			</code>
		),
	}),
	columnHelper.accessor("detected", {
		header: "Status",
		cell: (info) => {
			const detected = info.getValue();
			return (
				<Badge variant={detected ? "success" : "muted"}>
					{detected ? "Detected" : "Not detected"}
				</Badge>
			);
		},
	}),
	columnHelper.accessor("configPath", {
		header: "Config Path",
		cell: (info) => (
			<span className="font-mono text-xs text-muted-foreground">
				{info.getValue() ?? "—"}
			</span>
		),
	}),
]);

export function AgentsTable({ agents }: { agents: AgentSummary[] }) {
	const table = useTable({
		features,
		columns,
		data: agents,
	});

	return (
		<div className="overflow-x-auto rounded-xl border border-border bg-card">
			<table className="w-full text-left text-sm">
				<thead className="border-b border-border bg-slate-900/60 text-xs uppercase tracking-wider text-muted-foreground">
					{table.getHeaderGroups().map((group) => (
						<tr key={group.id}>
							{group.headers.map((header) => (
								<th key={header.id} className="px-4 py-3 font-medium">
									{header.isPlaceholder ? null : (
										<table.FlexRender header={header} />
									)}
								</th>
							))}
						</tr>
					))}
				</thead>
				<tbody className="divide-y divide-border">
					{table.getRowModel().rows.length === 0 ? (
						<tr>
							<td
								colSpan={columns.length}
								className="px-4 py-6 text-center text-muted-foreground"
							>
								No agents match the current filter.
							</td>
						</tr>
					) : (
						table.getRowModel().rows.map((row) => (
							<tr key={row.id} className="hover:bg-slate-900/40">
								{row.getAllCells().map((cell) => (
									<td key={cell.id} className="px-4 py-3">
										<table.FlexRender cell={cell} />
									</td>
								))}
							</tr>
						))
					)}
				</tbody>
			</table>
		</div>
	);
}
