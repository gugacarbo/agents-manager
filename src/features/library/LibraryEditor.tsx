import { useEffect, useState } from "react";
import { CodeEditor } from "@/components/CodeEditor";
import { Button } from "@/components/ui/button";
import type { SecretSpan } from "@/lib/editor-secrets";
import { type Category, type LibraryItem, library } from "@/lib/library";
export function LibraryEditor({
	item,
	onClose,
	onSaved,
}: {
	item: LibraryItem | null;
	onClose: () => void;
	onSaved: () => Promise<void>;
}) {
	const [name, setName] = useState(item?.name ?? "");
	const [category, setCategory] = useState<Category>(item?.category ?? "mcps");
	const [content, setContent] = useState("");
	const [spans, setSpans] = useState<SecretSpan[]>([]);
	const [reveal, setReveal] = useState(false);
	const [confirmed, setConfirmed] = useState(false);
	const [needsConfirmation, setNeedsConfirmation] = useState(false);
	const [error, setError] = useState("");
	useEffect(() => {
		if (item)
			library
				.read(item.id)
				.then((document) => {
					setContent(document.content);
					setSpans(document.secretSpans);
				})
				.catch((err) => setError(library.message(err)));
	}, [item]);
	async function save() {
		try {
			await library.save({
				itemId: item?.id,
				category,
				name,
				content,
				confirmLiteralSecrets: confirmed,
			});
			await onSaved();
		} catch (err) {
			if (err === "SecretConfirmationRequired") {
				setNeedsConfirmation(true);
				setError("");
			} else setError(library.message(err));
		}
	}
	return (
		<section
			aria-label="Library item editor"
			className="space-y-3 rounded-lg border border-border bg-card p-4"
		>
			<h2 className="font-semibold">{item ? "Edit item" : "Create item"}</h2>
			<label className="block text-sm">
				Name
				<input
					className="min-h-9 w-full rounded-md border border-input bg-background px-3"
					value={name}
					onChange={(e) => setName(e.target.value)}
				/>
			</label>
			<label className="block text-sm">
				Category
				<select
					disabled={!!item}
					className="min-h-9 w-full rounded-md border border-input bg-background px-3"
					value={category}
					onChange={(e) => setCategory(e.target.value as Category)}
				>
					<option value="mcps">MCPs</option>
					<option value="skills">Skills</option>
					<option value="rules">Rules</option>
					<option value="settings">Settings</option>
				</select>
			</label>
			<Button variant="outline" onClick={() => setReveal(!reveal)}>
				{reveal ? "Hide secrets" : "Reveal secrets"}
			</Button>
			<CodeEditor
				ariaLabel="Library item content"
				value={content}
				onChange={(value) => {
					setContent(value);
					setConfirmed(false);
				}}
				language={
					category === "mcps" || category === "settings" ? "json" : "markdown"
				}
				secretSpans={spans}
				revealSecrets={reveal}
			/>
			{needsConfirmation && (
				<label className="flex items-start gap-2 text-sm">
					<input
						type="checkbox"
						checked={confirmed}
						onChange={(e) => setConfirmed(e.target.checked)}
					/>
					This item contains literal secrets. Consider using {`\${VAR}`}; I
					confirm storing them.
				</label>
			)}
			{error && (
				<p role="alert" className="text-destructive">
					{error}
				</p>
			)}
			<div className="flex gap-2">
				<Button
					disabled={
						!name.trim() || !content || (needsConfirmation && !confirmed)
					}
					onClick={() => void save()}
				>
					Save item
				</Button>
				<Button variant="outline" onClick={onClose}>
					Cancel
				</Button>
			</div>
		</section>
	);
}
