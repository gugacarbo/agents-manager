import { useEffect, useId, useRef, useState } from "react";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
	NativeSelect,
	NativeSelectOption,
} from "@/components/ui/native-select";
import {
	type EnvironmentDraft,
	environments,
	type SshEnvironment,
	type SshInventory,
	validateEnvironment,
} from "@/lib/environments";

export function ConnectionForm({
	initial,
	onClose,
	onSaved,
	onChange,
}: {
	initial: SshEnvironment | null;
	onClose: () => void;
	onSaved: (item: SshEnvironment) => void;
	onChange: () => void;
}) {
	const formId = useId();
	const [name, setName] = useState(initial?.name ?? "");
	const [mode, setMode] = useState<"alias" | "manual">(
		initial?.connection.mode ?? "alias",
	);
	const [alias, setAlias] = useState(
		initial?.connection.mode === "alias" ? initial.connection.alias : "",
	);
	const [host, setHost] = useState(
		initial?.connection.mode === "manual" ? initial.connection.host : "",
	);
	const [user, setUser] = useState(
		initial?.connection.mode === "manual" ? initial.connection.user : "",
	);
	const [port, setPort] = useState(
		initial?.connection.mode === "manual"
			? String(initial.connection.port)
			: "22",
	);
	const [error, setError] = useState("");
	const [result, setResult] = useState<SshInventory | null>(null);
	const [busy, setBusy] = useState(false);
	const token = useRef(0);
	useEffect(
		() => () => {
			token.current++;
		},
		[],
	);
	function changed() {
		token.current++;
		setResult(null);
		setError("");
		setBusy(false);
		onChange();
	}
	function value(): EnvironmentDraft {
		return {
			name,
			connection:
				mode === "alias"
					? { mode, alias }
					: { mode, host, user, port: Number(port) },
		};
	}
	async function run(save: boolean) {
		const draft = value();
		const invalid = validateEnvironment(draft);
		if (invalid) {
			setError(invalid);
			return;
		}
		const current = ++token.current;
		setBusy(true);
		setError("");
		try {
			if (save) {
				const item = await environments.save(initial?.id ?? null, draft);
				if (current === token.current) onSaved(item);
			} else {
				const found = await environments.test(draft);
				if (current === token.current) setResult(found);
			}
		} catch (err) {
			if (current === token.current) setError(environments.message(err));
		} finally {
			if (current === token.current) setBusy(false);
		}
	}
	return (
		<section
			aria-label="SSH environment details"
			className="space-y-3 rounded-lg border border-border bg-card p-4"
		>
			<h2 className="font-semibold">
				{initial ? "Edit SSH environment" : "Add SSH environment"}
			</h2>
			<p className="text-sm text-muted-foreground">
				Authentication and host key approval must be configured in OpenSSH
				outside this app.
			</p>
			<div className="space-y-1.5">
				<Label htmlFor={`${formId}-name`}>Display name</Label>
				<Input
					id={`${formId}-name`}
					value={name}
					onChange={(event) => {
						changed();
						setName(event.target.value);
					}}
				/>
			</div>
			<div className="space-y-1.5">
				<Label htmlFor={`${formId}-mode`}>Connection mode</Label>
				<NativeSelect
					id={`${formId}-mode`}
					wrapperClassName="w-full"
					value={mode}
					onChange={(event) => {
						changed();
						setMode(event.target.value as "alias" | "manual");
					}}
				>
					<NativeSelectOption value="alias">OpenSSH alias</NativeSelectOption>
					<NativeSelectOption value="manual">
						Host, user and port
					</NativeSelectOption>
				</NativeSelect>
			</div>
			{mode === "alias" ? (
				<div className="space-y-1.5">
					<Label htmlFor={`${formId}-alias`}>SSH alias</Label>
					<Input
						id={`${formId}-alias`}
						value={alias}
						onChange={(event) => {
							changed();
							setAlias(event.target.value);
						}}
					/>
				</div>
			) : (
				<div className="grid gap-2 sm:grid-cols-3">
					<div className="space-y-1.5">
						<Label htmlFor={`${formId}-host`}>Host</Label>
						<Input
							id={`${formId}-host`}
							value={host}
							onChange={(event) => {
								changed();
								setHost(event.target.value);
							}}
						/>
					</div>
					<div className="space-y-1.5">
						<Label htmlFor={`${formId}-user`}>User</Label>
						<Input
							id={`${formId}-user`}
							value={user}
							onChange={(event) => {
								changed();
								setUser(event.target.value);
							}}
						/>
					</div>
					<div className="space-y-1.5">
						<Label htmlFor={`${formId}-port`}>Port</Label>
						<Input
							id={`${formId}-port`}
							type="number"
							min="1"
							max="65535"
							value={port}
							onChange={(event) => {
								changed();
								setPort(event.target.value);
							}}
						/>
					</div>
				</div>
			)}
			{error && (
				<Alert variant="destructive">
					<AlertDescription>{error}</AlertDescription>
				</Alert>
			)}
			{result && (
				<p role="status">
					Connected to {result.user}@{result.host}:{result.port}.
				</p>
			)}
			<div className="flex flex-wrap gap-2">
				<Button
					variant="outline"
					disabled={busy}
					onClick={() => void run(false)}
				>
					Test connection
				</Button>
				<Button disabled={busy} onClick={() => void run(true)}>
					Save environment
				</Button>
				<Button variant="ghost" onClick={onClose}>
					Cancel
				</Button>
			</div>
		</section>
	);
}
