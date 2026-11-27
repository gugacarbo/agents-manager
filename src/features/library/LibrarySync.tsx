import { useEffect, useId, useState } from "react";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Label } from "@/components/ui/label";
import {
	type LibraryItem,
	library,
	type SyncPreview,
	type SyncSelection,
} from "@/lib/library";
import { type Project, projects } from "@/lib/projects";
import { DeleteConfirmation } from "./DeleteConfirmation";
import { LinkedTargets } from "./LinkedTargets";
import { SyncPreviewList } from "./SyncPreviewList";

const agents = [
	["claude-code", "Claude Code"],
	["codex", "Codex"],
	["antigravity", "Antigravity"],
] as const;

function key(selection: SyncSelection) {
	return `${selection.agent}:${selection.scope}:${selection.projectId ?? ""}`;
}
export function LibrarySync({
	item,
	onEdit,
	onRefresh,
	onClose,
}: {
	item: LibraryItem;
	onEdit: () => void;
	onRefresh: () => Promise<void>;
	onClose: () => void;
}) {
	const id = useId();
	const [projectList, setProjectList] = useState<Project[]>([]);
	const [targets, setTargets] = useState<SyncSelection[]>([]);
	const [preview, setPreview] = useState<SyncPreview[] | null>(null);
	const [error, setError] = useState("");
	const [results, setResults] = useState<string[]>([]);
	const [deleting, setDeleting] = useState(false);
	const [busy, setBusy] = useState(false);
	useEffect(() => {
		projects
			.list()
			.then(setProjectList)
			.catch((err) => setError(library.message(err)));
	}, []);
	const options: SyncSelection[] = agents.flatMap(([agent]) => [
		{ agent, scope: "global" as const },
		...projectList
			.filter((project) => project.available)
			.map((project) => ({
				agent,
				scope: "project" as const,
				projectId: project.id,
			})),
	]);
	function toggle(option: SyncSelection) {
		setTargets((old) =>
			old.some((value) => key(value) === key(option))
				? old.filter((value) => key(value) !== key(option))
				: [...old, option],
		);
		setPreview(null);
		setResults([]);
	}
	async function inspect() {
		setBusy(true);
		try {
			const result = await library.preview(item.id, targets);
			if (
				result.length !== targets.length ||
				result.some(
					(value, index) =>
						value.agent !== targets[index]?.agent ||
						value.scope !== targets[index]?.scope,
				)
			)
				throw new Error(
					"The preview did not match the selected destinations. Please try again.",
				);
			setPreview(result);
			setError("");
		} catch (err) {
			setError(library.message(err));
		} finally {
			setBusy(false);
		}
	}
	async function apply() {
		if (!preview) return;
		setBusy(true);
		try {
			const output = await library.apply(
				item.id,
				targets.map((selection, index) => {
					const item = preview[index];
					if (!item) throw new Error("Missing preview for destination.");
					return { selection, expectedSha256: item.expectedSha256 };
				}),
			);
			setResults(
				output.map(
					(result) =>
						`${result.targetPath}: ${result.applied ? `Applied (${result.strategyUsed})` : `Failed: ${library.message(result.error)}`}`,
				),
			);
			setPreview(null);
			await onRefresh();
		} catch (err) {
			setError(library.message(err));
		} finally {
			setBusy(false);
		}
	}
	async function unlink(path: string) {
		try {
			await library.unlink(item.id, path);
			await onRefresh();
		} catch (err) {
			setError(library.message(err));
		}
	}
	async function remove() {
		try {
			await library.delete(item.id);
			await onRefresh();
			onClose();
		} catch (err) {
			setError(library.message(err));
		}
	}
	return (
		<section
			aria-label="Library item"
			className="space-y-4 rounded-lg border border-border bg-card p-4"
		>
			<div className="flex flex-wrap items-center justify-between gap-2">
				<div>
					<h2 className="font-semibold">{item.name}</h2>
					<p className="text-xs text-muted-foreground">
						{item.category} · {item.sourcePath}
					</p>
				</div>
				<div className="flex gap-2">
					<Button variant="outline" onClick={onEdit}>
						Edit item
					</Button>
					<Button variant="outline" onClick={() => setDeleting(!deleting)}>
						Delete item
					</Button>
				</div>
			</div>
			{deleting && (
				<DeleteConfirmation item={item} onConfirm={() => void remove()} />
			)}
			<LinkedTargets
				item={item}
				onInspect={(selection) => {
					setTargets(selection);
					setPreview(null);
				}}
				onUnlink={(path) => void unlink(path)}
			/>
			<fieldset disabled={busy} className="space-y-2">
				<legend className="font-medium">Sync destinations</legend>
				{options.map((option) => {
					const optId = `${id}-opt-${key(option)}`;
					const targetLabel = `${agents.find(([agent]) => agent === option.agent)?.[1]} · ${option.scope === "global" ? "Global" : projectList.find((project) => project.id === option.projectId)?.name}`;
					return (
						<div key={key(option)} className="flex items-center gap-2">
							<Checkbox
								id={optId}
								checked={targets.some((value) => key(value) === key(option))}
								onCheckedChange={() => toggle(option)}
							/>
							<Label htmlFor={optId} className="font-normal">
								{targetLabel}
							</Label>
						</div>
					);
				})}
				{(item.category === "skills" || item.category === "rules") &&
					targets.map((option) => {
						const copyId = `${id}-copy-${key(option)}`;
						const copyLabel = `Use copy for ${agents.find(([agent]) => agent === option.agent)?.[1]} · ${option.scope === "global" ? "Global" : projectList.find((project) => project.id === option.projectId)?.name} (explicit fallback)`;
						return (
							<div
								key={`copy:${key(option)}`}
								className="flex items-center gap-2"
							>
								<Checkbox
									id={copyId}
									checked={option.copy ?? false}
									onCheckedChange={(checked) => {
										setTargets((old) =>
											old.map((value) =>
												key(value) === key(option)
													? { ...value, copy: checked === true }
													: value,
											),
										);
										setPreview(null);
									}}
								/>
								<Label htmlFor={copyId} className="font-normal">
									{copyLabel}
								</Label>
							</div>
						);
					})}
			</fieldset>
			{error && (
				<Alert variant="destructive">
					<AlertDescription>{error}</AlertDescription>
				</Alert>
			)}
			<Button
				disabled={busy || targets.length === 0}
				onClick={() => void inspect()}
			>
				Preview sync
			</Button>
			{preview && (
				<>
					<SyncPreviewList items={preview} />
					<Button
						disabled={busy || !preview.some((value) => value.changed)}
						onClick={() => void apply()}
					>
						Apply sync
					</Button>
				</>
			)}
			{results.length > 0 && (
				<ul role="status" className="space-y-1 text-sm">
					{results.map((result) => (
						<li key={result}>{result}</li>
					))}
				</ul>
			)}
		</section>
	);
}
