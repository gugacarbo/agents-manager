import { useId, useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
export function SettingsForm({
	settings,
	onChange,
}: {
	settings: Record<string, string>;
	onChange: (settings: Record<string, string>) => void;
}) {
	const id = useId();
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
				<div key={name} className="space-y-1.5">
					<Label htmlFor={`${id}-setting-${name}`}>{name}</Label>
					<Input
						id={`${id}-setting-${name}`}
						className="font-mono"
						type={reveal ? "text" : "password"}
						value={value}
						onChange={(event) =>
							onChange({ ...settings, [name]: event.target.value })
						}
					/>
				</div>
			))}
			<div className="flex items-end gap-2">
				<div className="flex-1 space-y-1.5">
					<Label htmlFor={`${id}-new-key`}>New setting key</Label>
					<Input
						id={`${id}-new-key`}
						value={key}
						onChange={(event) => setKey(event.target.value)}
					/>
				</div>
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
