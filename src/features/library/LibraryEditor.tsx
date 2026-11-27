import { useEffect, useId, useState } from "react";
import { CodeEditor } from "@/components/CodeEditor";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
	NativeSelect,
	NativeSelectOption,
} from "@/components/ui/native-select";
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
	const id = useId();
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
			<div className="space-y-1.5">
				<Label htmlFor={`${id}-name`}>Name</Label>
				<Input
					id={`${id}-name`}
					value={name}
					onChange={(e) => setName(e.target.value)}
				/>
			</div>
			<div className="space-y-1.5">
				<Label htmlFor={`${id}-category`}>Category</Label>
				<NativeSelect
					id={`${id}-category`}
					wrapperClassName="w-full"
					disabled={!!item}
					value={category}
					onChange={(e) => setCategory(e.target.value as Category)}
				>
					<NativeSelectOption value="mcps">MCPs</NativeSelectOption>
					<NativeSelectOption value="skills">Skills</NativeSelectOption>
					<NativeSelectOption value="rules">Rules</NativeSelectOption>
					<NativeSelectOption value="settings">Settings</NativeSelectOption>
				</NativeSelect>
			</div>
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
				<div className="flex items-start gap-2">
					<Checkbox
						id={`${id}-confirm`}
						checked={confirmed}
						onCheckedChange={(checked) => setConfirmed(checked === true)}
					/>
					<Label htmlFor={`${id}-confirm`} className="font-normal">
						This item contains literal secrets. Consider using {`\${VAR}`}; I
						confirm storing them.
					</Label>
				</div>
			)}
			{error && (
				<Alert variant="destructive">
					<AlertDescription>{error}</AlertDescription>
				</Alert>
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
