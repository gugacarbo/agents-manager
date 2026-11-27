import { Chunk } from "@codemirror/merge";
import { Text } from "@codemirror/state";
import { RotateCcw } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { CodeEditor } from "@/components/CodeEditor";
import { PageHeader } from "@/components/patterns";
import { Button } from "@/components/ui/button";

const INITIAL_ORIGINAL = `{
  "mcpServers": {
    "filesystem": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-filesystem", "/workspace"]
    }
  }
}`;

const INITIAL_DRAFT = `{
  "mcpServers": {
    "filesystem": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-filesystem", "/home/demo"]
    }
  }
}`;

const originalDoc = Text.of(INITIAL_ORIGINAL.split("\n"));

function changedLines(doc: Text, from: number, end: number, sign: string) {
	return doc
		.sliceString(from, end)
		.split("\n")
		.map((line) => `${sign} ${line}`)
		.join("\n");
}

function SampleDiff({ draft }: { draft: string }) {
	const edited = Text.of(draft.split("\n"));
	const chunks = Chunk.build(originalDoc, edited, {
		scanLimit: 500,
		timeout: 30,
	});
	return (
		<section
			aria-label="Changes from original"
			className="demo-editor-height overflow-auto rounded-lg border border-border bg-card p-3"
		>
			{chunks.length === 0 ? (
				<p className="text-sm text-muted-foreground">
					No changes from the original.
				</p>
			) : (
				<div className="space-y-3 font-mono text-sm">
					{chunks.map((chunk) => (
						<div key={`${chunk.fromA}:${chunk.fromB}`}>
							{chunk.fromA !== chunk.toA && (
								<pre className="diff-removed whitespace-pre-wrap wrap-anywhere px-2 py-1">
									{changedLines(originalDoc, chunk.fromA, chunk.endA, "-")}
								</pre>
							)}
							{chunk.fromB !== chunk.toB && (
								<pre className="diff-added whitespace-pre-wrap wrap-anywhere px-2 py-1">
									{changedLines(edited, chunk.fromB, chunk.endB, "+")}
								</pre>
							)}
						</div>
					))}
				</div>
			)}
		</section>
	);
}

function useIsWideViewport(): boolean {
	const [isWide, setIsWide] = useState(() => {
		if (typeof window === "undefined") {
			return true;
		}
		return window.innerWidth >= 1024;
	});

	useEffect(() => {
		function handleResize() {
			setIsWide(window.innerWidth >= 1024);
		}
		window.addEventListener("resize", handleResize);
		return () => window.removeEventListener("resize", handleResize);
	}, []);

	return isWide;
}

export function DiffDemoPage() {
	const [draft, setDraft] = useState(INITIAL_DRAFT);
	const [activeTab, setActiveTab] = useState<"editor" | "diff">("editor");
	const isWide = useIsWideViewport();
	const editorTabRef = useRef<HTMLButtonElement>(null);
	const diffTabRef = useRef<HTMLButtonElement>(null);
	const isDirty = draft !== INITIAL_DRAFT;

	function handleTabKeyDown(event: React.KeyboardEvent<HTMLButtonElement>) {
		let next: "editor" | "diff";
		switch (event.key) {
			case "ArrowRight":
			case "ArrowLeft":
				next = activeTab === "editor" ? "diff" : "editor";
				break;
			case "Home":
				next = "editor";
				break;
			case "End":
				next = "diff";
				break;
			default:
				return;
		}
		event.preventDefault();
		setActiveTab(next);
		(next === "editor" ? editorTabRef : diffTabRef).current?.focus();
	}

	return (
		<div className="space-y-4">
			<PageHeader
				title="Diff demo"
				description="Sample JSON only. No local files are changed. The draft is discarded when you leave this demo."
				actions={
					<Button
						variant="outline"
						size="sm"
						disabled={!isDirty}
						onClick={() => setDraft(INITIAL_DRAFT)}
					>
						<RotateCcw className="size-3.5" aria-hidden="true" />
						<span>Reset sample</span>
					</Button>
				}
			/>

			{isWide ? (
				<div className="grid gap-4 lg:grid-cols-2">
					<div className="min-w-0">
						<h3 className="mb-2 text-sm font-medium">Sample editor</h3>
						<CodeEditor
							value={draft}
							onChange={setDraft}
							ariaLabel="Sample editor"
							className="demo-editor-height"
						/>
					</div>
					<div className="min-w-0">
						<h3 className="mb-2 text-sm font-medium">Changes from original</h3>
						<SampleDiff draft={draft} />
					</div>
				</div>
			) : (
				<>
					<div
						role="tablist"
						aria-label="Diff demo views"
						className="flex flex-wrap gap-2 border-b border-border pb-2"
					>
						{(["editor", "diff"] as const).map((panel) => (
							<Button
								key={panel}
								ref={panel === "editor" ? editorTabRef : diffTabRef}
								id={`demo-tab-${panel}`}
								variant="ghost"
								size="sm"
								role="tab"
								aria-controls={`demo-panel-${panel}`}
								aria-selected={activeTab === panel}
								tabIndex={activeTab === panel ? 0 : -1}
								onClick={() => setActiveTab(panel)}
								onKeyDown={handleTabKeyDown}
								className="aria-selected:bg-selection aria-selected:text-selection-foreground"
							>
								{panel === "editor" ? "Sample editor" : "Changes from original"}
							</Button>
						))}
					</div>
					<div
						id="demo-panel-editor"
						role="tabpanel"
						aria-labelledby="demo-tab-editor"
						hidden={activeTab !== "editor"}
						className="min-w-0"
					>
						<CodeEditor
							value={draft}
							onChange={setDraft}
							ariaLabel="Sample editor"
							className="demo-editor-height"
						/>
					</div>
					<div
						id="demo-panel-diff"
						role="tabpanel"
						aria-labelledby="demo-tab-diff"
						hidden={activeTab !== "diff"}
						className="min-w-0"
					>
						{activeTab === "diff" && <SampleDiff draft={draft} />}
					</div>
				</>
			)}
		</div>
	);
}
