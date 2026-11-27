import {
	ArrowUpRight,
	FileText,
	Plug,
	Puzzle,
	SlidersHorizontal,
} from "lucide-react";
import { useEffect, useState } from "react";
import { ErrorState, LoadingState, PageHeader } from "@/components/patterns";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { type ConfigDocument, type ConfigEntry, configs } from "@/lib/configs";
import { type Project, projects } from "@/lib/projects";
import type { AgentSummary } from "@/lib/schemas";
import { ConfigEditor } from "./ConfigEditor";

const names = {
	"claude-code": "Claude Code",
	codex: "Codex",
	antigravity: "Antigravity",
};
const categories = ["mcps", "skills", "rules", "settings"] as const;
const labels = {
	mcps: "MCPs",
	skills: "Skills",
	rules: "Rules",
	settings: "Settings",
};
const categoryIcons = {
	mcps: Plug,
	skills: Puzzle,
	rules: FileText,
	settings: SlidersHorizontal,
};

export function AgentConfigsPage({
	agentKind,
	projectId,
}: {
	agentKind: AgentSummary["kind"];
	projectId?: string;
}) {
	const [project, setProject] = useState<Project | null>(null);
	const [entries, setEntries] = useState<ConfigEntry[]>([]);
	const [doc, setDoc] = useState<ConfigDocument | null>(null);
	const [error, setError] = useState("");
	const [loading, setLoading] = useState(true);
	useEffect(() => {
		let active = true;
		setLoading(true);
		setDoc(null);
		setProject(null);
		if (projectId)
			projects
				.list()
				.then((items) => {
					if (active)
						setProject(items.find((item) => item.id === projectId) ?? null);
				})
				.catch((err) => {
					if (active) setError(projects.message(err));
				});
		configs
			.listForScope(projectId, agentKind)
			.then((result) => {
				if (active) setEntries(result);
			})
			.catch((err) => {
				if (active) setError(configs.message(err));
			})
			.finally(() => {
				if (active) setLoading(false);
			});
		return () => {
			active = false;
		};
	}, [agentKind, projectId]);
	async function open(entry: ConfigEntry) {
		try {
			setError("");
			const document = await configs.read(entry.path);
			setDoc({ ...document, entry });
		} catch (err) {
			setError(configs.message(err));
		}
	}
	return (
		<div className="space-y-6">
			<PageHeader
				title={`${names[agentKind]} · ${projectId ? (project?.name ?? "Project") : "Global"}`}
				description={
					project
						? `${project.rootPath} · Project configuration`
						: "Inspect and edit native configuration files."
				}
			/>
			{error && (
				<ErrorState title="Could not open configuration" message={error} />
			)}
			{loading ? (
				<LoadingState message="Loading configuration files…" />
			) : (
				<div className="grid gap-4 sm:grid-cols-2">
					{categories.map((category) => {
						const Icon = categoryIcons[category];
						const files = entries.filter(
							(entry) => entry.category === category,
						);
						return (
							<section
								key={category}
								aria-label={labels[category]}
								className="min-w-0 rounded-xl border border-border bg-card"
							>
								<h2 className="flex items-center gap-2.5 border-b border-border px-4 py-3 text-sm font-semibold">
									<Icon
										className="size-4 text-muted-foreground"
										aria-hidden="true"
									/>
									{labels[category]}
								</h2>
								{files.length === 0 ? (
									<p className="px-4 py-5 text-sm text-muted-foreground">
										No files found.
									</p>
								) : (
									<ul className="divide-y divide-border">
										{files.map((entry) => (
											<ConfigFileRow
												key={entry.path}
												entry={entry}
												selected={entry.path === doc?.entry.path}
												onOpen={() => void open(entry)}
											/>
										))}
									</ul>
								)}
							</section>
						);
					})}
				</div>
			)}
			{doc && (
				<ConfigEditor
					key={`${doc.entry.category}:${doc.entry.path}:${doc.sha256}`}
					initial={doc}
				/>
			)}
		</div>
	);
}

function ConfigFileRow({
	entry,
	selected,
	onOpen,
}: {
	entry: ConfigEntry;
	selected: boolean;
	onOpen: () => void;
}) {
	const file = (
		<span className="flex min-w-0 flex-col gap-0.5">
			<span className="font-medium wrap-anywhere">{entry.label}</span>
			<code className="text-xs leading-relaxed text-muted-foreground wrap-anywhere">
				{entry.path}
			</code>
		</span>
	);
	return (
		<li className="space-y-2 px-3 py-2.5">
			{entry.exists ? (
				<Button
					variant="ghost"
					aria-current={selected || undefined}
					className="h-auto w-full justify-between px-2 py-2 text-left whitespace-normal aria-current:bg-selection aria-current:text-selection-foreground"
					onClick={onOpen}
				>
					{file}
					<ArrowUpRight className="size-3.5 shrink-0" aria-hidden="true" />
				</Button>
			) : (
				<div className="px-2 pt-2 text-sm">{file}</div>
			)}
			<div className="flex flex-wrap gap-2 px-2 pb-1">
				<Badge variant="muted" className="font-mono uppercase">
					{entry.format}
				</Badge>
				{!entry.exists && <Badge variant="muted">Not found</Badge>}
			</div>
		</li>
	);
}
