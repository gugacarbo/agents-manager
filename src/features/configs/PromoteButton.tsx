import { useId, useState } from "react";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import type { ConfigDocument } from "@/lib/configs";
import { library } from "@/lib/library";
export function PromoteButton({ document }: { document: ConfigDocument }) {
	const id = useId();
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
					<div className="space-y-1.5">
						<Label htmlFor={`${id}-name`}>Library item name</Label>
						<Input
							id={`${id}-name`}
							value={name}
							onChange={(event) => setName(event.target.value)}
						/>
					</div>
					{needsConfirmation && (
						<div className="flex items-start gap-2">
							<Checkbox
								id={`${id}-confirm`}
								checked={confirmed}
								onCheckedChange={(checked) => setConfirmed(checked === true)}
							/>
							<Label htmlFor={`${id}-confirm`} className="font-normal">
								This file contains literal secrets. Consider using {`\${VAR};`}{" "}
								I confirm storing them in the library.
							</Label>
						</div>
					)}
					{error && (
						<Alert variant="destructive">
							<AlertDescription>{error}</AlertDescription>
						</Alert>
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
