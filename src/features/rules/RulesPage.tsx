import { Chunk } from "@codemirror/merge";
import { Text } from "@codemirror/state";
import { useBlocker } from "@tanstack/react-router";
import { FileText, FolderOpen } from "lucide-react";
import { useState } from "react";
import { CodeEditor } from "@/components/CodeEditor";
import { ErrorState, PageHeader } from "@/components/patterns";
import { Button } from "@/components/ui/button";
import { isTauriRuntime } from "@/lib/agents";
import {
	type RuleReview,
	type RuleScope,
	type RuleSnapshot,
	rules,
} from "@/lib/rules";

function RuleDiff({ review }: { review: RuleReview }) {
	const original = Text.of(review.original.split("\n"));
	const edited = Text.of(review.content.split("\n"));
	const chunks = Chunk.build(original, edited, { scanLimit: 500, timeout: 30 });
	return (
		<section
			className="max-h-80 overflow-auto rounded-lg border border-border bg-card p-3 font-mono text-sm"
			// biome-ignore lint/a11y/noNoninteractiveTabindex: keyboard users must be able to scroll the diff.
			tabIndex={0}
			aria-label="Reviewed changes"
		>
			{chunks.length === 0 && <p>No visible text changes.</p>}
			{chunks.map((chunk) => (
				<div key={`${chunk.fromA}:${chunk.fromB}`}>
					{chunk.fromA !== chunk.toA && (
						<pre className="diff-removed whitespace-pre-wrap wrap-anywhere">
							{original
								.sliceString(chunk.fromA, chunk.endA)
								.split("\n")
								.map((line) => `- ${line}`)
								.join("\n")}
						</pre>
					)}
					{chunk.fromB !== chunk.toB && (
						<pre className="diff-added whitespace-pre-wrap wrap-anywhere">
							{edited
								.sliceString(chunk.fromB, chunk.endB)
								.split("\n")
								.map((line) => `+ ${line}`)
								.join("\n")}
						</pre>
					)}
				</div>
			))}
		</section>
	);
}

export function RulesPage() {
	const [active, setActive] = useState<{
		scope: RuleScope;
		file: RuleSnapshot;
	}>();
	const [draft, setDraft] = useState("");
	const [review, setReview] = useState<RuleReview>();
	const [busy, setBusy] = useState(false);
	const [error, setError] = useState<string>();
	const [status, setStatus] = useState("");
	const dirty = !!active && draft !== active.file.content;
	useBlocker({
		shouldBlockFn: () =>
			busy || (dirty && !window.confirm("Discard your unsaved changes?")),
		enableBeforeUnload: dirty || busy,
	});

	function load(scope: RuleScope, reload = false) {
		if (dirty && !window.confirm("Discard your unsaved changes?")) return;
		void run(async () => {
			const file =
				reload && active
					? await rules.reload(active.file.version)
					: await rules.open(scope);
			if (!file) return;
			setActive({ scope, file });
			setDraft(file.content);
			setReview(undefined);
		});
	}

	async function run(operation: () => Promise<void>) {
		setBusy(true);
		setError(undefined);
		setStatus("");
		try {
			await operation();
		} catch (reason) {
			setReview(undefined);
			setError(rules.message(reason));
		} finally {
			setBusy(false);
		}
	}

	if (!isTauriRuntime())
		return (
			<PageHeader
				title="Edit Claude Code rules"
				description="Open the desktop app to edit local rules."
			/>
		);

	return (
		<div className="space-y-6" aria-busy={busy}>
			<PageHeader
				title="Edit Claude Code rules"
				description="Open an existing CLAUDE.md file. The editor and diff show its contents, which may be sensitive. A backup is stored locally before saving."
			/>
			<fieldset
				className="flex flex-wrap gap-2 rounded-xl border border-border bg-card p-4 sm:p-5"
				aria-label="Choose rule scope"
			>
				<Button
					variant="outline"
					disabled={busy}
					onClick={() => load("global")}
				>
					<FileText className="size-4" aria-hidden="true" />
					Open global rules
				</Button>
				<Button
					variant="outline"
					disabled={busy}
					onClick={() => load("project")}
				>
					<FolderOpen className="size-4" aria-hidden="true" />
					Choose project folder
				</Button>
			</fieldset>
			{busy && (
				<p role="status" className="text-sm text-muted-foreground">
					Working…
				</p>
			)}
			{error && (
				<ErrorState
					title="Could not complete the rule operation"
					message={error}
				/>
			)}
			{status && (
				<p role="status" className="text-sm">
					{status}
				</p>
			)}
			{active && (
				<section
					aria-label="Rule editor"
					className="space-y-4 rounded-xl border border-border bg-card p-4 sm:p-5"
				>
					<div className="flex flex-wrap items-center justify-between gap-3 border-b border-border pb-4">
						<div className="min-w-0 space-y-1">
							<h2 className="font-semibold">
								{active.scope === "global" ? "Global" : "Project"} · CLAUDE.md
							</h2>
							<code className="wrap-anywhere text-sm text-muted-foreground">
								{active.file.path}
							</code>
						</div>
						<Button
							variant="outline"
							size="sm"
							disabled={busy}
							onClick={() => load(active.scope, true)}
						>
							Reload file
						</Button>
					</div>
					<CodeEditor
						key={active.file.path}
						value={draft}
						onChange={(value) => {
							setDraft(value);
							setReview(undefined);
							setStatus("");
						}}
						readOnly={busy}
						language="markdown"
						ariaLabel="Claude Code rules"
						className="demo-editor-height"
					/>
					<Button
						disabled={!dirty || busy}
						onClick={() =>
							void run(async () =>
								setReview(await rules.review(active.file.version, draft)),
							)
						}
					>
						Review changes
					</Button>
					{review && (
						<section className="space-y-3" aria-label="Review before saving">
							<h3 className="font-medium">Review before saving</h3>
							<p className="wrap-anywhere font-mono text-sm text-muted-foreground">
								{active.file.path}
							</p>
							<RuleDiff review={review} />
							<Button
								disabled={busy}
								onClick={() =>
									void run(async () => {
										const file = await rules.save(review.reviewId);
										setActive({ ...active, file });
										setDraft(file.content);
										setReview(undefined);
										setStatus("Rules saved.");
									})
								}
							>
								Save changes
							</Button>
						</section>
					)}
				</section>
			)}
		</div>
	);
}
