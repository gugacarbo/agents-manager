import { useState } from "react";
import { Button } from "@/components/ui/button";
import { envVar, type Server } from "@/lib/structured";

const inputClass =
	"min-h-9 w-full rounded-md border border-input bg-background px-3";
export function McpForm({
	servers,
	existingNames = [],
	onChange,
}: {
	servers: Server[];
	existingNames?: string[];
	onChange: (servers: Server[]) => void;
}) {
	const [reveal, setReveal] = useState(false);
	function update(index: number, patch: Partial<Server>) {
		onChange(
			servers.map((server, i) =>
				i === index ? { ...server, ...patch } : server,
			),
		);
	}
	return (
		<div className="space-y-4">
			<div className="flex items-center justify-between">
				<h3 className="font-medium">MCP servers</h3>
				<Button
					variant="outline"
					size="sm"
					onClick={() =>
						onChange([
							...servers,
							{
								name: "",
								transport: "stdio",
								command: "",
								args: [],
								env: [],
								enabled: true,
							},
						])
					}
				>
					Add server
				</Button>
			</div>
			{servers.length === 0 && (
				<p className="text-sm text-muted-foreground">
					No MCP servers configured.
				</p>
			)}
			{servers.map((server, index) => (
				<fieldset
					// biome-ignore lint/suspicious/noArrayIndexKey: Entries are append-only while editing.
					key={index}
					className="space-y-3 rounded-md border border-border p-3"
				>
					<legend className="px-1 text-sm">Server {index + 1}</legend>
					<label className="block text-sm">
						Name
						<input
							className={inputClass}
							value={server.name}
							readOnly={existingNames.includes(server.name)}
							onChange={(event) => update(index, { name: event.target.value })}
						/>
					</label>
					<label className="block text-sm">
						Transport
						<select
							className={inputClass}
							value={server.transport}
							onChange={(event) =>
								update(index, {
									transport: event.target.value as Server["transport"],
									...(event.target.value === "http"
										? { command: undefined, args: undefined }
										: { url: undefined }),
								})
							}
						>
							<option value="stdio">Standard input/output</option>
							<option value="http">HTTP</option>
						</select>
					</label>
					{server.transport === "stdio" ? (
						<>
							<label className="block text-sm">
								Command
								<input
									className={inputClass}
									value={server.command ?? ""}
									onChange={(event) =>
										update(index, { command: event.target.value })
									}
								/>
							</label>
							<label className="block text-sm">
								Arguments (one per line)
								<textarea
									className={inputClass}
									value={(server.args ?? []).join("\n")}
									onChange={(event) =>
										update(index, { args: event.target.value.split("\n") })
									}
								/>
							</label>
						</>
					) : (
						<label className="block text-sm">
							URL
							<input
								className={inputClass}
								value={server.url ?? ""}
								onChange={(event) => update(index, { url: event.target.value })}
							/>
						</label>
					)}
					<label className="flex items-center gap-2 text-sm">
						<input
							type="checkbox"
							checked={server.enabled}
							onChange={(event) =>
								update(index, { enabled: event.target.checked })
							}
						/>
						Enabled
					</label>
					<div className="flex items-center justify-between">
						<span className="text-sm font-medium">Environment</span>
						<div className="flex gap-2">
							<Button
								variant="outline"
								size="sm"
								onClick={() => setReveal(!reveal)}
							>
								{reveal ? "Hide" : "Reveal"}
							</Button>
							<Button
								variant="outline"
								size="sm"
								onClick={() =>
									update(index, { env: [...server.env, envVar("", "")] })
								}
							>
								Add variable
							</Button>
						</div>
					</div>
					{server.env.map((variable, envIndex) => (
						<div
							// biome-ignore lint/suspicious/noArrayIndexKey: Entries are append-only while editing.
							key={envIndex}
							className="grid gap-2 sm:grid-cols-2"
						>
							<label className="text-sm">
								Key
								<input
									className={inputClass}
									aria-label={`Environment key ${envIndex + 1}`}
									value={variable.key}
									onChange={(event) =>
										update(index, {
											env: server.env.map((item, i) =>
												i === envIndex
													? envVar(event.target.value, item.value)
													: item,
											),
										})
									}
								/>
							</label>
							<label className="text-sm">
								Value
								<input
									className={inputClass}
									aria-label={`Environment value ${envIndex + 1}`}
									type={reveal ? "text" : "password"}
									value={variable.value}
									onChange={(event) =>
										update(index, {
											env: server.env.map((item, i) =>
												i === envIndex
													? envVar(item.key, event.target.value)
													: item,
											),
										})
									}
								/>
							</label>
						</div>
					))}
					{server.env.some(
						(variable) => variable.value && !variable.isEnvRef,
					) && (
						<p className="text-xs text-muted-foreground">
							Consider an environment variable reference such as {`\${VAR}`}{" "}
							instead of a literal value.
						</p>
					)}
				</fieldset>
			))}
		</div>
	);
}
