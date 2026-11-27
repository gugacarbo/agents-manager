import { type ReactNode, useEffect, useRef, useState } from "react";
import { Button } from "@/components/ui/button";
import { RemoteConfigBrowser } from "@/features/configs";
import {
	environments,
	type SshEnvironment,
	type SshInventory,
} from "@/lib/environments";
import { ConnectionForm } from "./ConnectionForm";
export function EnvironmentSwitcher({ children }: { children: ReactNode }) {
	const [items, setItems] = useState<SshEnvironment[]>([]);
	const [active, setActive] = useState("local");
	const [form, setForm] = useState<SshEnvironment | "new" | null>(null);
	const [inventory, setInventory] = useState<SshInventory | null>(null);
	const [error, setError] = useState("");
	const [loading, setLoading] = useState(false);
	const [version, setVersion] = useState(0);
	const [confirmRemove, setConfirmRemove] = useState(false);
	const sequence = useRef(0);
	const listSequence = useRef(0);
	useEffect(() => {
		let live = true;
		const listToken = listSequence.current;
		environments
			.list()
			.then((value) => {
				if (live && listToken === listSequence.current) setItems(value);
			})
			.catch((err) => {
				if (live && listToken === listSequence.current)
					setError(environments.message(err));
			});
		return () => {
			live = false;
		};
	}, []);
	useEffect(() => {
		void version;
		if (active === "local" || form !== null) return;
		const token = ++sequence.current;
		setInventory(null);
		setError("");
		setLoading(true);
		environments
			.inspect(active)
			.then((value) => {
				if (token === sequence.current) setInventory(value);
			})
			.catch((err) => {
				if (token === sequence.current) setError(environments.message(err));
			})
			.finally(() => {
				if (token === sequence.current) setLoading(false);
			});
		return () => {
			sequence.current++;
		};
	}, [active, version, form]);
	function switchTo(id: string) {
		sequence.current++;
		setActive(id);
		setForm(null);
		setInventory(null);
		setError("");
		setLoading(false);
		setConfirmRemove(false);
	}
	const selected = items.find((item) => item.id === active);
	async function remove() {
		if (!selected) return;
		const id = selected.id;
		const token = ++sequence.current;
		listSequence.current++;
		setInventory(null);
		setLoading(false);
		try {
			await environments.remove(id);
			setItems((old) => old.filter((item) => item.id !== id));
			setActive((current) => (current === id ? "local" : current));
			if (token === sequence.current) {
				setForm(null);
				setConfirmRemove(false);
				setError("");
			}
		} catch (err) {
			if (token === sequence.current) setError(environments.message(err));
		}
	}
	return (
		<div className="space-y-4">
			<div className="flex flex-wrap items-end gap-2 rounded-lg border border-border bg-card p-3">
				<label className="min-w-48 flex-1 text-sm">
					Environment
					<select
						className="min-h-9 w-full rounded-md border border-input bg-background px-3"
						value={active}
						onChange={(event) => switchTo(event.target.value)}
					>
						<option value="local">Local</option>
						{items.map((item) => (
							<option key={item.id} value={item.id}>
								{item.name}
							</option>
						))}
					</select>
				</label>
				<Button
					variant="outline"
					onClick={() => {
						sequence.current++;
						setForm("new");
						setInventory(null);
						setLoading(false);
					}}
				>
					Add SSH environment
				</Button>
				{selected && (
					<>
						<Button
							variant="outline"
							onClick={() => {
								sequence.current++;
								setInventory(null);
								setLoading(false);
								setForm(selected);
							}}
						>
							Edit environment
						</Button>
						<Button
							variant="outline"
							onClick={() => setConfirmRemove(!confirmRemove)}
						>
							Remove environment
						</Button>
					</>
				)}
			</div>
			{confirmRemove && selected && (
				<div className="space-y-2 rounded-md border border-border p-3">
					<p>
						Remove {selected.name} from this app? The server will not be
						changed.
					</p>
					<Button onClick={() => void remove()}>Confirm removal</Button>
				</div>
			)}
			{form && (
				<ConnectionForm
					key={form === "new" ? "new" : form.id}
					initial={form === "new" ? null : form}
					onClose={() => {
						setForm(null);
						setVersion((value) => value + 1);
					}}
					onChange={() => {
						sequence.current++;
						setInventory(null);
						setLoading(false);
					}}
					onSaved={(item) => {
						listSequence.current++;
						sequence.current++;
						setItems((old) => [
							...old.filter((value) => value.id !== item.id),
							item,
						]);
						setForm(null);
						setActive(item.id);
						setVersion((value) => value + 1);
						setError("");
					}}
				/>
			)}
			{active === "local" ? (
				children
			) : (
				<section
					aria-label="Remote agent inventory"
					className="space-y-4 rounded-lg border border-border bg-card p-4"
				>
					<div className="flex flex-wrap justify-between gap-2">
						<div>
							<h1 className="text-xl font-semibold">
								{selected?.name ?? "SSH environment"}
							</h1>
							<p className="text-sm text-muted-foreground">
								SSH · Agent detection checks executables in the non-interactive
								PATH and in ~/.local/bin. Agent sign-in is not checked.
							</p>
						</div>
						<Button
							variant="outline"
							disabled={loading || !!form}
							onClick={() => setVersion((value) => value + 1)}
						>
							Refresh inventory
						</Button>
					</div>
					{loading && <p role="status">Checking remote agent inventory…</p>}
					{error && (
						<p role="alert" className="text-destructive">
							{error}
						</p>
					)}
					{inventory && (
						<>
							<p className="font-mono text-sm wrap-anywhere">
								{inventory.user}@{inventory.host}:{inventory.port}
							</p>
							<ul className="divide-y divide-border">
								{(
									[
										["Claude Code", inventory.claude],
										["Codex", inventory.codex],
									] as const
								).map(([name, found]) => (
									<li key={name} className="flex justify-between gap-2 py-2">
										<span>{name}</span>
										<span>{found ? "Detected" : "Not detected"}</span>
									</li>
								))}
							</ul>
							<RemoteConfigBrowser
								key={active}
								environmentId={active}
								claude={inventory.claude}
								codex={inventory.codex}
							/>
						</>
					)}
				</section>
			)}
			{active === "local" && error && (
				<p role="alert" className="text-destructive">
					{error}
				</p>
			)}
		</div>
	);
}
