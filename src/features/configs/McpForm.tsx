import { useId, useState } from "react";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
	NativeSelect,
	NativeSelectOption,
} from "@/components/ui/native-select";
import { Textarea } from "@/components/ui/textarea";
import { envVar, type Server } from "@/lib/structured";

export function McpForm({
	servers,
	existingNames = [],
	onChange,
}: {
	servers: Server[];
	existingNames?: string[];
	onChange: (servers: Server[]) => void;
}) {
	const formId = useId();
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
			{servers.map((server, index) => {
				const prefix = `${formId}-srv-${index}`;
				return (
					<fieldset
						// biome-ignore lint/suspicious/noArrayIndexKey: Entries are append-only while editing.
						key={index}
						className="space-y-3 rounded-md border border-border p-3"
					>
						<legend className="px-1 text-sm">Server {index + 1}</legend>
						<div className="space-y-1.5">
							<Label htmlFor={`${prefix}-name`}>Name</Label>
							<Input
								id={`${prefix}-name`}
								value={server.name}
								readOnly={existingNames.includes(server.name)}
								onChange={(event) =>
									update(index, { name: event.target.value })
								}
							/>
						</div>
						<div className="space-y-1.5">
							<Label htmlFor={`${prefix}-transport`}>Transport</Label>
							<NativeSelect
								id={`${prefix}-transport`}
								wrapperClassName="w-full"
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
								<NativeSelectOption value="stdio">
									Standard input/output
								</NativeSelectOption>
								<NativeSelectOption value="http">HTTP</NativeSelectOption>
							</NativeSelect>
						</div>
						{server.transport === "stdio" ? (
							<>
								<div className="space-y-1.5">
									<Label htmlFor={`${prefix}-command`}>Command</Label>
									<Input
										id={`${prefix}-command`}
										value={server.command ?? ""}
										onChange={(event) =>
											update(index, { command: event.target.value })
										}
									/>
								</div>
								<div className="space-y-1.5">
									<Label htmlFor={`${prefix}-args`}>
										Arguments (one per line)
									</Label>
									<Textarea
										id={`${prefix}-args`}
										value={(server.args ?? []).join("\n")}
										onChange={(event) =>
											update(index, { args: event.target.value.split("\n") })
										}
									/>
								</div>
							</>
						) : (
							<div className="space-y-1.5">
								<Label htmlFor={`${prefix}-url`}>URL</Label>
								<Input
									id={`${prefix}-url`}
									value={server.url ?? ""}
									onChange={(event) =>
										update(index, { url: event.target.value })
									}
								/>
							</div>
						)}
						<div className="flex items-center gap-2">
							<Checkbox
								id={`${prefix}-enabled`}
								checked={server.enabled}
								onCheckedChange={(checked) =>
									update(index, { enabled: checked === true })
								}
							/>
							<Label htmlFor={`${prefix}-enabled`} className="font-normal">
								Enabled
							</Label>
						</div>
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
								<div className="space-y-1.5">
									<Label htmlFor={`${prefix}-key-${envIndex}`}>Key</Label>
									<Input
										id={`${prefix}-key-${envIndex}`}
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
								</div>
								<div className="space-y-1.5">
									<Label htmlFor={`${prefix}-val-${envIndex}`}>Value</Label>
									<Input
										id={`${prefix}-val-${envIndex}`}
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
								</div>
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
				);
			})}
		</div>
	);
}
