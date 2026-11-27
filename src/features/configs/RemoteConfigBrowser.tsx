import { useState } from "react";
import { Button } from "@/components/ui/button";
import { RemoteConfigEditor } from "./RemoteConfigEditor";
export function RemoteConfigBrowser({
	environmentId,
	claude,
	codex,
}: {
	environmentId: string;
	claude: boolean;
	codex: boolean;
}) {
	const [agent, setAgent] = useState<"claude-code" | "codex" | null>(null);
	return (
		<div className="space-y-3">
			<div className="flex flex-wrap gap-2">
				{claude && (
					<Button variant="outline" onClick={() => setAgent("claude-code")}>
						Open Claude Code configurations
					</Button>
				)}
				{codex && (
					<Button variant="outline" onClick={() => setAgent("codex")}>
						Open Codex configurations
					</Button>
				)}
			</div>
			{agent && (
				<RemoteConfigEditor
					key={`${environmentId}:${agent}`}
					environmentId={environmentId}
					agent={agent}
				/>
			)}
		</div>
	);
}
