import { Chunk } from "@codemirror/merge";
import { Text } from "@codemirror/state";
import { RotateCcw } from "lucide-react";
import { useEffect, useState } from "react";
import { CodeEditor } from "@/components/CodeEditor";
import { PageHeader } from "@/components/patterns";
import { Button } from "@/components/ui/button";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";

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
	const isDirty = draft !== INITIAL_DRAFT;

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
				<Tabs
					value={activeTab}
					onValueChange={(value) => {
						if (value === "editor" || value === "diff") {
							setActiveTab(value);
						}
					}}
				>
					<TabsList aria-label="Diff demo views">
						<TabsTrigger id="demo-tab-editor" value="editor">
							Sample editor
						</TabsTrigger>
						<TabsTrigger id="demo-tab-diff" value="diff">
							Changes from original
						</TabsTrigger>
					</TabsList>
					<TabsContent
						id="demo-panel-editor"
						value="editor"
						forceMount
						className={activeTab === "editor" ? "min-w-0" : "hidden"}
					>
						<CodeEditor
							value={draft}
							onChange={setDraft}
							ariaLabel="Sample editor"
							className="demo-editor-height"
						/>
					</TabsContent>
					<TabsContent id="demo-panel-diff" value="diff" className="min-w-0">
						<SampleDiff draft={draft} />
					</TabsContent>
				</Tabs>
			)}
		</div>
	);
}
