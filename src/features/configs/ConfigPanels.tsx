import { useRef } from "react";
import { CodeEditor, DiffViewer } from "@/components/CodeEditor";
import { Button } from "@/components/ui/button";
import type { ConfigDocument } from "@/lib/configs";
import { maskedConfig } from "@/lib/editor-secrets";
import { StructuredEditor, type StructuredPatch } from "./StructuredEditor";

type Props = {
	base: ConfigDocument;
	draft: string;
	maskedDraft: string;
	editorSpans: ConfigDocument["secretSpans"];
	reveal: boolean;
	busy: boolean;
	conflict: boolean;
	mode: "raw" | "structured";
	source: string;
	tab: "editor" | "diff";
	setTab: (value: "editor" | "diff") => void;
	preview: (patch: StructuredPatch, toRaw: boolean) => Promise<void>;
	setStructuredDirty: (value: boolean) => void;
	setDraft: (value: string) => void;
	setStructured: (value: null) => void;
	setMaskedDraft: (value: string) => void;
};
export function ConfigPanels({
	base,
	draft,
	maskedDraft,
	editorSpans,
	reveal,
	busy,
	conflict,
	mode,
	source,
	tab,
	setTab,
	preview,
	setStructuredDirty,
	setDraft,
	setStructured,
	setMaskedDraft,
}: Props) {
	const editorTab = useRef<HTMLButtonElement>(null);
	const diffTab = useRef<HTMLButtonElement>(null);
	function tabKey(event: React.KeyboardEvent<HTMLButtonElement>) {
		let next: "editor" | "diff";
		if (event.key === "Home") next = "editor";
		else if (event.key === "End") next = "diff";
		else if (event.key === "ArrowRight" || event.key === "ArrowLeft")
			next = tab === "editor" ? "diff" : "editor";
		else return;
		event.preventDefault();
		setTab(next);
		(next === "editor" ? editorTab : diffTab).current?.focus();
	}
	return (
		<>
			<div
				role="tablist"
				aria-label="Configuration views"
				className="flex gap-2 border-b border-border pb-2 lg:hidden"
			>
				{(["editor", "diff"] as const).map((panel) => (
					<Button
						key={panel}
						ref={panel === "editor" ? editorTab : diffTab}
						role="tab"
						id={`config-tab-${panel}`}
						aria-controls={`config-panel-${panel}`}
						aria-selected={tab === panel}
						tabIndex={tab === panel ? 0 : -1}
						variant="ghost"
						onClick={() => setTab(panel)}
						onKeyDown={tabKey}
						className="aria-selected:bg-selection aria-selected:text-selection-foreground"
					>
						{panel === "editor" ? "Editor" : "Diff"}
					</Button>
				))}
			</div>
			<div className="grid gap-4 lg:grid-cols-2">
				<div
					id="config-panel-editor"
					role="tabpanel"
					aria-labelledby="config-tab-editor"
					className={tab === "editor" ? "min-w-0" : "hidden min-w-0 lg:block"}
				>
					<h3 className="mb-2 text-sm font-medium">Editor</h3>
					{mode === "structured" &&
					base.entry.format !== "markdown" &&
					(base.entry.category === "mcps" ||
						base.entry.category === "settings") ? (
						<StructuredEditor
							content={source}
							format={base.entry.format}
							category={base.entry.category}
							busy={busy || conflict}
							onPreview={preview}
							onChange={() => setStructuredDirty(true)}
						/>
					) : (
						<CodeEditor
							value={draft}
							readOnly={busy}
							onChange={(content, masked) => {
								setDraft(content);
								setStructured(null);
								setMaskedDraft(masked ?? content);
							}}
							language={base.entry.format}
							secretSpans={editorSpans}
							revealSecrets={reveal}
							className="demo-editor-height"
						/>
					)}
				</div>
				<div
					id="config-panel-diff"
					role="tabpanel"
					aria-labelledby="config-tab-diff"
					className={tab === "diff" ? "min-w-0" : "hidden min-w-0 lg:block"}
				>
					<h3 className="mb-2 text-sm font-medium">Diff against disk</h3>
					<DiffViewer
						original={
							reveal
								? base.content
								: maskedConfig(base.content, base.secretSpans)
						}
						modified={reveal ? draft : maskedDraft}
						language={base.entry.format}
						className="demo-editor-height"
					/>
				</div>
			</div>
		</>
	);
}
