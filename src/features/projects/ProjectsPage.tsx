import { Link } from "@tanstack/react-router";
import { ArrowUpRight, FolderOpen, FolderPlus } from "lucide-react";
import { useEffect, useState } from "react";
import {
	EmptyState,
	ErrorState,
	LoadingState,
	PageHeader,
} from "@/components/patterns";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { type Project, projects } from "@/lib/projects";

export function ProjectsPage({ preview = false }: { preview?: boolean }) {
	const [items, setItems] = useState<Project[]>([]);
	const [path, setPath] = useState("");
	const [error, setError] = useState("");
	const [loading, setLoading] = useState(true);
	useEffect(() => {
		if (preview) return;
		let active = true;
		projects
			.list()
			.then((data) => {
				if (active) setItems(data);
			})
			.catch((err) => {
				if (active) setError(projects.message(err));
			})
			.finally(() => {
				if (active) setLoading(false);
			});
		return () => {
			active = false;
		};
	}, [preview]);
	async function add(event: React.FormEvent) {
		event.preventDefault();
		try {
			const item = await projects.register(path);
			setItems((old) => [
				...old.filter((project) => project.id !== item.id),
				item,
			]);
			setPath("");
			setError("");
		} catch (err) {
			setError(projects.message(err));
		}
	}
	async function remove(id: string) {
		try {
			await projects.remove(id);
			setItems((old) => old.filter((project) => project.id !== id));
			setError("");
		} catch (err) {
			setError(projects.message(err));
		}
	}
	return (
		<div className="space-y-6">
			<PageHeader
				title="Projects"
				description="Register folders to inspect agent configuration within each project."
			/>
			{preview ? (
				<EmptyState
					title="Projects are managed in the desktop app"
					description="Open Agents Manager on your desktop to register a folder and inspect its agent configuration."
				/>
			) : (
				<>
					<form
						onSubmit={(event) => void add(event)}
						className="flex flex-wrap items-end gap-3 rounded-xl border border-border bg-card p-4 sm:p-5"
					>
						<label className="flex w-full min-w-0 flex-col gap-2 text-sm font-medium sm:flex-1">
							Project folder path
							<input
								className="min-h-10 rounded-md border border-input bg-background px-3 font-mono text-xs font-normal placeholder:text-muted-foreground"
								value={path}
								onChange={(event) => setPath(event.target.value)}
								placeholder="/home/user/project"
								required
							/>
						</label>
						<Button type="submit" className="h-10">
							<FolderPlus className="size-4" aria-hidden="true" />
							Add project
						</Button>
					</form>
					{error && (
						<ErrorState
							title="Could not complete project operation"
							message={error}
						/>
					)}
					{loading ? (
						<LoadingState message="Loading projects…" />
					) : items.length === 0 ? (
						!error && (
							<EmptyState
								title="No projects registered yet."
								description="Add a folder to browse its agent configuration."
								className="border-dashed bg-background"
							/>
						)
					) : (
						<ul
							aria-label="Registered projects"
							className="divide-y divide-border rounded-xl border border-border bg-card"
						>
							{items.map((item) => (
								<li key={item.id} className="space-y-4 p-4 sm:p-5">
									<div className="flex flex-wrap items-center justify-between gap-2">
										<div className="flex min-w-0 items-center gap-3">
											<span className="grid size-10 shrink-0 place-items-center rounded-md bg-selection text-selection-foreground">
												<FolderOpen className="size-4" aria-hidden="true" />
											</span>
											<div className="min-w-0 space-y-0.5">
												<h2 className="font-semibold wrap-anywhere">
													{item.name}
												</h2>
												<p className="font-mono text-xs text-muted-foreground wrap-anywhere">
													{item.rootPath}
												</p>
											</div>
										</div>
										<div className="flex flex-wrap items-center gap-2">
											{!item.available && (
												<Badge variant="muted">Unavailable</Badge>
											)}
											<Button
												variant="outline"
												size="sm"
												onClick={() => void remove(item.id)}
												aria-label={`Remove from list: ${item.name}`}
											>
												Remove from list
											</Button>
										</div>
									</div>
									{item.available && (
										<nav
											aria-label={`Agents in ${item.name}`}
											className="flex flex-wrap gap-2 border-t border-border pt-3"
										>
											{(
												[
													["claude-code", "Claude Code"],
													["codex", "Codex"],
													["antigravity", "Antigravity"],
												] as const
											).map(([kind, name]) => (
												<Link
													key={kind}
													to="/projects/$projectId/agents/$agentKind"
													params={{ projectId: item.id, agentKind: kind }}
													className="flex min-h-8 items-center gap-1.5 rounded-md border border-border px-2.5 text-xs font-medium transition-colors hover:bg-accent"
												>
													{name}
													<ArrowUpRight
														className="size-3 text-muted-foreground"
														aria-hidden="true"
													/>
												</Link>
											))}
										</nav>
									)}
								</li>
							))}
						</ul>
					)}
				</>
			)}
		</div>
	);
}
