import { useState } from "react";
import { Button } from "@/components/ui/button";
import { parseStructured, type Server } from "@/lib/structured";
import { McpForm } from "./McpForm";
import { SettingsForm } from "./SettingsForm";
export type StructuredPatch = {
	mcpServers?: Server[];
	settingsPatch?: Record<string, unknown>;
};
export function StructuredEditor({
	content,
	format,
	category,
	busy,
	onPreview,
	onChange,
}: {
	content: string;
	format: "json" | "toml";
	category: "mcps" | "settings";
	busy: boolean;
	onPreview: (patch: StructuredPatch, toRaw: boolean) => Promise<void>;
	onChange: () => void;
}) {
	const [initial] = useState(() => parseStructured(content, format));
	const [servers, setServers] = useState(initial.servers);
	const [settings, setSettings] = useState(() =>
		Object.fromEntries(
			Object.entries(initial.settings).map(([key, value]) => [
				key,
				JSON.stringify(value),
			]),
		),
	);
	const [error, setError] = useState("");
	async function preview(toRaw: boolean) {
		try {
			const patch: StructuredPatch = {};
			if (category === "mcps") {
				if (
					servers.some((server) => !server.name.trim()) ||
					new Set(servers.map((server) => server.name)).size !== servers.length
				)
					throw new Error("Server names must be present and unique.");
				patch.mcpServers = servers.filter(
					(server) =>
						JSON.stringify(server) !==
						JSON.stringify(
							initial.servers.find((item) => item.name === server.name),
						),
				);
			} else {
				patch.settingsPatch = Object.fromEntries(
					Object.entries(settings)
						.filter(
							([key, value]) => value !== JSON.stringify(initial.settings[key]),
						)
						.map(([key, value]) => [key, JSON.parse(value)]),
				);
			}
			setError("");
			await onPreview(patch, toRaw);
		} catch (err) {
			setError(err instanceof Error ? err.message : String(err));
		}
	}
	return (
		<div className="space-y-4 rounded-lg border border-border bg-card p-4">
			<fieldset disabled={busy}>
				{category === "mcps" ? (
					<McpForm
						existingNames={initial.servers.map((server) => server.name)}
						servers={servers}
						onChange={(value) => {
							setServers(value);
							onChange();
						}}
					/>
				) : (
					<SettingsForm
						settings={settings}
						onChange={(value) => {
							setSettings(value);
							onChange();
						}}
					/>
				)}
			</fieldset>
			{error && (
				<p role="alert" className="text-destructive">
					{error}
				</p>
			)}
			<div className="flex flex-wrap gap-2">
				<Button disabled={busy} onClick={() => void preview(false)}>
					Preview changes
				</Button>
				<Button
					variant="outline"
					disabled={busy}
					onClick={() => void preview(true)}
				>
					Switch to Raw
				</Button>
			</div>
		</div>
	);
}
