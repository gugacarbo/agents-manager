import { useEffect, useRef, useState } from "react";
import { CodeEditor } from "@/components/CodeEditor";
import { Button } from "@/components/ui/button";
import { validateConfig } from "@/lib/config-validation";
import {
	assertRemoteEntry,
	protectedProjects,
	type RemoteDocument,
	type RemoteEntry,
	type RemoteReview as RemoteReviewData,
	remoteConfigs,
	reviewMatches,
	sameRemoteTarget,
} from "@/lib/remote-configs";

import { RemoteFileList } from "./RemoteFileList";
import { RemoteReview } from "./RemoteReview";
export function RemoteConfigEditor({
	environmentId,
	agent,
}: {
	environmentId: string;
	agent: "claude-code" | "codex";
}) {
	const [entries, setEntries] = useState<RemoteEntry[]>([]);
	const [editorSpans, setEditorSpans] = useState<RemoteDocument["secretSpans"]>(
		[],
	);
	const [listing, setListing] = useState(true);
	const [doc, setDoc] = useState<RemoteDocument | null>(null);
	const [draft, setDraft] = useState("");
	const [review, setReview] = useState<RemoteReviewData | null>(null);
	const [reveal, setReveal] = useState(false);
	const [error, setError] = useState("");
	const [state, setState] = useState<
		| "idle"
		| "busy"
		| "conflict"
		| "unconfirmed"
		| "destination-changed"
		| "failed"
	>("idle");
	const [notice, setNotice] = useState("");
	const sequence = useRef(0);
	useEffect(() => {
		const token = ++sequence.current;
		setEntries([]);
		setListing(true);
		setDoc(null);
		remoteConfigs
			.list(environmentId, agent)
			.then((items) => {
				if (token === sequence.current) {
					setEntries(items);
					setListing(false);
				}
			})
			.catch((err) => {
				if (token === sequence.current) {
					setError(remoteConfigs.message(err));
					setListing(false);
				}
			});
		return () => {
			sequence.current++;
		};
	}, [environmentId, agent]);
	async function open(entry: RemoteEntry) {
		const token = ++sequence.current;
		setDoc(null);
		setReview(null);
		setState("busy");
		setError("");
		setNotice("");
		try {
			const current = await remoteConfigs.read(environmentId, entry.id);
			if (token !== sequence.current) return;
			assertRemoteEntry(current, entry, agent);
			setDoc(current);
			setEditorSpans(current.secretSpans);
			setDraft(current.content);
			setState("idle");
		} catch (err) {
			if (token === sequence.current) {
				setError(remoteConfigs.message(err));
				setState("idle");
			}
		}
	}
	async function preview() {
		if (!doc) return;
		const token = ++sequence.current;
		setReview(null);
		setState("busy");
		setError("");
		try {
			const result = await remoteConfigs.review(doc.handle, draft);
			if (token === sequence.current) {
				if (!reviewMatches(result, doc, draft))
					throw "RemoteDestinationChanged";
				setReview(result);
				setState("idle");
			}
		} catch (err) {
			if (token === sequence.current) {
				setError(remoteConfigs.message(err));
				setState(
					err === "ExternalConflict"
						? "conflict"
						: err === "RemoteDestinationChanged"
							? "destination-changed"
							: "idle",
				);
			}
		}
	}
	async function save() {
		if (!review) return;
		const token = ++sequence.current;
		const reviewId = review.reviewId;
		setReview(null);
		setState("busy");
		setError("");
		try {
			await remoteConfigs.save(reviewId);
			if (token === sequence.current) {
				setNotice("Remote save confirmed. Reopen the file to edit again.");
				setState("destination-changed");
			}
		} catch (err) {
			if (token !== sequence.current) return;
			if (err === "ExternalConflict") setState("conflict");
			else if (err === "RemoteUnconfirmed") setState("unconfirmed");
			else if (err === "RemoteDestinationChanged")
				setState("destination-changed");
			else setState("failed");
			setError(remoteConfigs.message(err));
		}
	}
	async function reread() {
		if (!doc) return;
		const oldState = state;
		const token = ++sequence.current;
		setState("busy");
		setError("");
		try {
			const current = await remoteConfigs.verify(doc.handle);
			if (token !== sequence.current) return;
			if (!sameRemoteTarget(current, doc) || current.entry.id !== doc.entry.id)
				throw "RemoteDestinationChanged";
			setDoc(current);
			setReview(null);
			setState("idle");
			setNotice(
				oldState === "unconfirmed"
					? current.content === draft
						? "The current remote content matches the draft. The previous save result was not confirmed."
						: "The current remote content differs from the draft. Review again before saving."
					: "Loaded the current remote version; your draft is preserved.",
			);
		} catch (err) {
			if (token === sequence.current) {
				setError(remoteConfigs.message(err));
				setState(
					err === "RemoteDestinationChanged" ? "destination-changed" : oldState,
				);
			}
		}
	}
	const syntax = doc ? validateConfig(draft, doc.entry.format) : null;
	const protectedChange = doc
		? protectedProjects(doc.content, draft, doc.entry)
		: false;
	return (
		<div className="space-y-3 border-t border-border pt-4">
			<RemoteFileList
				entries={entries}
				listing={listing}
				busy={state === "busy"}
				onOpen={(entry) => void open(entry)}
			/>
			{error && (
				<p role="alert" className="text-destructive">
					{error}
				</p>
			)}
			{notice && <p role="status">{notice}</p>}
			{doc && (
				<>
					<p className="font-mono text-sm wrap-anywhere">
						{doc.user}@{doc.host}:{doc.port} · {doc.path}
					</p>
					<Button variant="outline" onClick={() => setReveal(!reveal)}>
						{reveal ? "Hide secrets" : "Reveal secrets"}
					</Button>
					<CodeEditor
						ariaLabel="Remote configuration editor"
						value={draft}
						language={doc.entry.format}
						secretSpans={editorSpans}
						revealSecrets={reveal}
						readOnly={
							state === "busy" ||
							state === "unconfirmed" ||
							state === "destination-changed"
						}
						onChange={(value) => {
							sequence.current++;
							setDraft(value);
							setReview(null);
							setNotice("");
						}}
					/>
					{syntax && (
						<p role="alert" className="text-destructive">
							Invalid {doc.entry.format}: {syntax}
						</p>
					)}
					{protectedChange && (
						<p role="alert" className="text-destructive">
							Project settings in this shared file cannot be changed remotely.
						</p>
					)}
					<RemoteReview
						state={state}
						notice={notice}
						review={review}
						reveal={reveal}
						format={doc.entry.format}
						onReread={() => void reread()}
						onSave={() => void save()}
					/>
					<Button
						variant="outline"
						disabled={
							state !== "idle" ||
							!!syntax ||
							protectedChange ||
							draft === doc.content
						}
						onClick={() => void preview()}
					>
						Review remote diff
					</Button>
				</>
			)}
		</div>
	);
}
