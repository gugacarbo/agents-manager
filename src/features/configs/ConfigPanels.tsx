import { CodeEditor, DiffViewer } from "@/components/CodeEditor";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
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
	return (
		<>
			<Tabs
				value={tab}
				onValueChange={(value) => {
					if (value === "editor" || value === "diff") {
						setTab(value);
					}
				}}
				className="lg:hidden"
			>
				<TabsList aria-label="Configuration views">
					<TabsTrigger id="config-tab-editor" value="editor">
						Editor
					</TabsTrigger>
					<TabsTrigger id="config-tab-diff" value="diff">
						Diff
					</TabsTrigger>
				</TabsList>
			</Tabs>
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
