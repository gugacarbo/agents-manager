import { useState } from "react";
import { Button } from "@/components/ui/button";
import type { ConfigDocument } from "@/lib/configs";
import { library } from "@/lib/library";
export function PromoteButton({ document }: { document: ConfigDocument }) {
	const [open, setOpen] = useState(false);
	const [name, setName] = useState(document.entry.label);
	const [confirmed, setConfirmed] = useState(false);
	const [needsConfirmation, setNeedsConfirmation] = useState(
		document.secretSpans.length > 0,
	);
	const [error, setError] = useState("");
	const [done, setDone] = useState(false);
	async function promote() {
		try {
			await library.promote({
				path: document.entry.path,
				category: document.entry.category,
				name,
				confirmLiteralSecrets: confirmed,
			});
			setDone(true);
			setOpen(false);
			setError("");
		} catch (err) {
			if (err === "SecretConfirmationRequired") {
				setNeedsConfirmation(true);
				setConfirmed(false);
			} else setError(library.message(err));
		}
	}
	return (
		<div className="space-y-2">
			<Button variant="outline" onClick={() => setOpen(!open)}>
				Promote to library
			</Button>
			{done && <p role="status">Added to library.</p>}
			{open && (
				<div className="space-y-2 rounded-lg border border-border p-3">
					<p className="text-sm">The saved file on disk will be promoted.</p>
					<label className="block text-sm">
						Library item name
						<input
							className="min-h-9 w-full rounded-md border border-input bg-background px-3"
							value={name}
							onChange={(event) => setName(event.target.value)}
						/>
					</label>
					{needsConfirmation && (
						<label className="flex items-start gap-2 text-sm">
							<input
								type="checkbox"
								checked={confirmed}
								onChange={(event) => setConfirmed(event.target.checked)}
							/>
							This file contains literal secrets. Consider using {`\${VAR}`}; I
							confirm storing them in the library.
						</label>
					)}
					{error && (
						<p role="alert" className="text-destructive">
							{error}
						</p>
					)}
					<Button
						disabled={!name.trim() || (needsConfirmation && !confirmed)}
						onClick={() => void promote()}
					>
						Confirm promotion
					</Button>
				</div>
			)}
		</div>
	);
}
