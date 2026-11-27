import { useState } from "react";
import { Button } from "@/components/ui/button";
export function SettingsForm({
	settings,
	onChange,
}: {
	settings: Record<string, string>;
	onChange: (settings: Record<string, string>) => void;
}) {
	const [key, setKey] = useState("");
	const [reveal, setReveal] = useState(false);
	return (
		<div className="space-y-3">
			<div className="flex items-center justify-between">
				<h3 className="font-medium">Settings</h3>
				<Button variant="outline" size="sm" onClick={() => setReveal(!reveal)}>
					{reveal ? "Hide values" : "Reveal values"}
				</Button>
			</div>
			<p className="text-xs text-muted-foreground">
				Values use JSON syntax. Existing keys and nested objects are preserved.
			</p>
			{Object.entries(settings).map(([name, value]) => (
				<label key={name} className="block text-sm">
					{name}
					<input
						className="min-h-9 w-full rounded-md border border-input bg-background px-3 font-mono"
						type={reveal ? "text" : "password"}
						value={value}
						onChange={(event) =>
							onChange({ ...settings, [name]: event.target.value })
						}
					/>
				</label>
			))}
			<div className="flex items-end gap-2">
				<label className="flex-1 text-sm">
					New setting key
					<input
						className="min-h-9 w-full rounded-md border border-input bg-background px-3"
						value={key}
						onChange={(event) => setKey(event.target.value)}
					/>
				</label>
				<Button
					variant="outline"
					disabled={!key.trim() || key in settings}
					onClick={() => {
						onChange({ ...settings, [key]: '""' });
						setKey("");
					}}
				>
					Add setting
				</Button>
			</div>
		</div>
	);
}
