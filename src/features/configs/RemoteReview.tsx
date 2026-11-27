import { DiffViewer } from "@/components/CodeEditor";
import { Button } from "@/components/ui/button";
import { maskedConfig } from "@/lib/editor-secrets";
import type { RemoteReview as RemoteReviewData } from "@/lib/remote-configs";
export function RemoteReview({
	state,
	notice,
	review,
	reveal,
	format,
	onReread,
	onSave,
}: {
	state: string;
	notice: string;
	review: RemoteReviewData | null;
	reveal: boolean;
	format: "json" | "toml";
	onReread: () => void;
	onSave: () => void;
}) {
	return (
		<>
			{state === "conflict" && (
				<div role="alert" className="space-y-2">
					<p>
						The remote file changed. Reload the current version; your draft will
						be preserved.
					</p>
					<Button variant="outline" onClick={() => onReread()}>
						Reload from remote
					</Button>
				</div>
			)}
			{state === "failed" && (
				<div role="alert" className="space-y-2">
					<p>
						Save failed. Reload the current remote version before reviewing
						again; your draft will be preserved.
					</p>
					<Button variant="outline" onClick={onReread}>
						Reload from remote
					</Button>
				</div>
			)}
			{state === "unconfirmed" && (
				<div role="alert" className="space-y-2">
					<p>
						The save result is unconfirmed. Verify the current remote content
						before taking further action; no retry has been made.
					</p>
					<Button variant="outline" onClick={() => onReread()}>
						Verify remote content
					</Button>
				</div>
			)}
			{state === "destination-changed" && !notice && (
				<p role="alert">
					The remote destination changed. Open the file again before editing.
				</p>
			)}
			{review && (
				<section aria-label="Remote save review" className="space-y-2">
					<h3 className="font-medium">Review remote diff</h3>
					<p className="font-mono text-sm wrap-anywhere">
						{review.user}@{review.host}:{review.port} · {review.path}
					</p>
					<DiffViewer
						language={format}
						original={
							reveal
								? review.originalContent
								: maskedConfig(
										review.originalContent,
										review.originalSecretSpans,
									)
						}
						modified={
							reveal
								? review.proposedContent
								: maskedConfig(
										review.proposedContent,
										review.proposedSecretSpans,
									)
						}
					/>
					<Button onClick={() => onSave()}>Confirm remote save</Button>
				</section>
			)}
		</>
	);
}
