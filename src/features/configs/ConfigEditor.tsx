import { useEffect, useRef, useState } from "react";
import { Button } from "@/components/ui/button";
import { validateConfig } from "@/lib/config-validation";
import {
	type ConfigDocument,
	configs,
	isExternalConflict,
	structuredConfigs,
} from "@/lib/configs";
import { maskedConfig } from "@/lib/editor-secrets";
import { ConfigPanels } from "./ConfigPanels";
import { ConflictBanner } from "./ConflictBanner";
import type { StructuredPatch } from "./StructuredEditor";
export function ConfigEditor({ initial }: { initial: ConfigDocument }) {
	const [mode, setMode] = useState<"raw" | "structured">("raw");
	const [source, setSource] = useState(initial.content);
	const [structured, setStructured] = useState<{
		patch: StructuredPatch;
		source: string;
	} | null>(null);
	const [structuredDirty, setStructuredDirty] = useState(false);
	const [base, setBase] = useState(initial);
	const [editorSpans, setEditorSpans] = useState(initial.secretSpans);
	const [draft, setDraft] = useState(initial.content);
	const [maskedDraft, setMaskedDraft] = useState(() =>
		maskedConfig(initial.content, initial.secretSpans),
	);
	const [reveal, setReveal] = useState(false);
	const [conflict, setConflict] = useState(false);
	const [reviewing, setReviewing] = useState(false);
	const [error, setError] = useState("");
	const [busy, setBusy] = useState(false);
	const [tab, setTab] = useState<"editor" | "diff">("editor");
	const editorHeading = useRef<HTMLHeadingElement>(null);
	const baseRef = useRef(base);
	baseRef.current = base;
	const syntax = validateConfig(draft, base.entry.format);
	useEffect(() => editorHeading.current?.focus(), []);
	useEffect(() => {
		let active = true;
		let stop: (() => void) | undefined;
		configs
			.watch(initial.entry.path, (hash) => {
				if (active && hash !== baseRef.current.sha256) {
					setConflict(true);
					setReviewing(false);
				}
			})
			.then((unlisten) => {
				if (active) stop = unlisten;
				else unlisten();
			})
			.catch((err) => {
				if (active) setError(`Could not monitor file: ${configs.message(err)}`);
			});
		return () => {
			active = false;
			stop?.();
		};
	}, [initial.entry.path]);
	async function reload(review: boolean) {
		setBusy(true);
		try {
			const current = await configs.read(base.entry.path, base.entry);
			baseRef.current = current;
			setBase(current);
			if (!review) {
				setStructured(null);
				setStructuredDirty(false);
				setMode("raw");
				setDraft(current.content);
				setEditorSpans(current.secretSpans);
				setMaskedDraft(maskedConfig(current.content, current.secretSpans));
			}
			setConflict(false);
			setReviewing(review);
			setError("");
			if (review) setTab("diff");
		} catch (err) {
			setError(configs.message(err));
		} finally {
			setBusy(false);
		}
	}
	async function save() {
		setBusy(true);
		try {
			const result = structured
				? await structuredConfigs.save(
						{
							path: base.entry.path,
							expectedSha256: base.sha256,
							...structured.patch,
						},
						structured.source,
					)
				: await configs.save(base.entry.path, draft, base.sha256);
			const saved = { ...base, content: draft, sha256: result.sha256 };
			baseRef.current = saved;
			setBase(saved);
			const current = await configs.read(base.entry.path, base.entry);
			if (current.sha256 !== result.sha256) {
				setConflict(true);
				setReviewing(false);
				return;
			}
			baseRef.current = current;
			setBase(current);
			setEditorSpans(current.secretSpans);
			setMaskedDraft(maskedConfig(current.content, current.secretSpans));
			setConflict(false);
			setReviewing(false);
			setStructured(null);
			setStructuredDirty(false);
			setMode("raw");
			setError("");
		} catch (err) {
			if (isExternalConflict(err)) {
				setConflict(true);
				setReviewing(false);
			} else setError(configs.message(err));
		} finally {
			setBusy(false);
		}
	}
	async function preview(patch: StructuredPatch, toRaw: boolean) {
		setBusy(true);
		try {
			const result = await structuredConfigs.preview(
				{
					path: base.entry.path,
					expectedSha256: base.sha256,
					...patch,
				},
				source,
			);
			setDraft(result.content);
			setEditorSpans(result.secretSpans);
			setMaskedDraft(maskedConfig(result.content, result.secretSpans));
			setStructured({ patch, source });
			setStructuredDirty(false);
			setTab("diff");
			if (toRaw) setMode("raw");
			setError("");
		} catch (err) {
			if (isExternalConflict(err)) setConflict(true);
			else setError(configs.message(err));
		} finally {
			setBusy(false);
		}
	}
	const canSave =
		draft !== base.content && !syntax && !conflict && !busy && !structuredDirty;
	return (
		<section
			aria-label="Configuration editing"
			className="space-y-4 rounded-xl border border-border bg-card p-4 sm:p-5"
		>
			<div className="flex flex-wrap items-start justify-between gap-3 border-b border-border pb-4">
				<div className="min-w-0 space-y-1">
					<h2
						ref={editorHeading}
						tabIndex={-1}
						className="w-fit rounded-sm font-semibold wrap-anywhere"
					>
						{base.entry.label}
					</h2>
					<p className="font-mono text-xs text-muted-foreground wrap-anywhere">
						{base.entry.path}
					</p>
				</div>
				<div className="flex flex-wrap gap-2">
					<Button variant="outline" onClick={() => setReveal(!reveal)}>
						{reveal ? "Hide secrets" : "Reveal secrets"}
					</Button>
					<Button disabled={!canSave} onClick={() => void save()}>
						{reviewing ? "Confirm overwrite" : "Save"}
					</Button>
				</div>
			</div>
			{error && (
				<p role="alert" className="text-destructive">
					{error}
				</p>
			)}
			{conflict && (
				<ConflictBanner
					busy={busy}
					onReload={(review) => void reload(review)}
				/>
			)}
			{reviewing && (
				<p role="status">
					Review the diff against the latest disk version, then confirm
					overwrite. Further disk changes will require another review.
				</p>
			)}
			{(base.entry.category === "mcps" || base.entry.category === "settings") &&
				base.entry.format !== "markdown" &&
				mode === "raw" && (
					<Button
						variant="outline"
						disabled={busy || !!syntax}
						onClick={() => {
							setSource(draft);
							setStructuredDirty(true);
							setTab("editor");
							setMode("structured");
						}}
					>
						Structured
					</Button>
				)}
			{syntax && (
				<p role="alert" className="text-destructive">
					Invalid {base.entry.format}: {syntax}
				</p>
			)}
			<ConfigPanels
				base={base}
				draft={draft}
				maskedDraft={maskedDraft}
				editorSpans={editorSpans}
				reveal={reveal}
				busy={busy}
				conflict={conflict}
				mode={mode}
				source={source}
				tab={tab}
				setTab={setTab}
				preview={preview}
				setStructuredDirty={setStructuredDirty}
				setDraft={setDraft}
				setStructured={setStructured}
				setMaskedDraft={setMaskedDraft}
			/>
		</section>
	);
}
