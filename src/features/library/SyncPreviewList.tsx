import { useState } from "react";
import { DiffViewer } from "@/components/CodeEditor";
import { Button } from "@/components/ui/button";
import { maskedConfig } from "@/lib/editor-secrets";
import type { SyncPreview } from "@/lib/library";
export function SyncPreviewList({ items }: { items: SyncPreview[] }) {
	const [reveal, setReveal] = useState(false);
	return (
		<section aria-label="Sync preview" className="space-y-3">
			<div className="flex items-center justify-between">
				<h3 className="font-medium">Preview sync</h3>
				<Button variant="outline" size="sm" onClick={() => setReveal(!reveal)}>
					{reveal ? "Hide secrets" : "Reveal secrets"}
				</Button>
			</div>
			{items.map((item) => (
				<article
					key={item.targetPath}
					className="space-y-2 rounded-md border border-border p-3"
				>
					<p className="text-sm font-medium wrap-anywhere">
						{item.agent} · {item.scope} · {item.targetPath}
					</p>
					<p className="text-xs text-muted-foreground">
						Strategy: {item.strategy} ·{" "}
						{item.changed ? "Changes pending" : "No changes"} · Expected
						SHA-256: {item.expectedSha256 ?? "absent"}
					</p>
					{item.warnings.map((warning) => (
						<p key={warning} className="text-sm text-destructive">
							{warning}
						</p>
					))}
					<DiffViewer
						original={
							reveal
								? (item.currentContent ?? "")
								: maskedConfig(
										item.currentContent ?? "",
										item.currentSecretSpans,
									)
						}
						modified={
							reveal
								? item.proposedContent
								: maskedConfig(item.proposedContent, item.proposedSecretSpans)
						}
						language="markdown"
					/>
				</article>
			))}
		</section>
	);
}
