import { useQuery, useQueryClient } from "@tanstack/react-query";
import {
	createHashHistory,
	createRootRoute,
	createRoute,
	createRouter,
	Link,
	Outlet,
	type RouterHistory,
	useNavigate,
} from "@tanstack/react-router";
import { Command, FileCode2, LayoutGrid, RefreshCw } from "lucide-react";
import { useMemo, useState } from "react";
import { toast } from "sonner";
import { AgentCard } from "@/components/AgentCard";
import { CodeEditor, DiffViewer } from "@/components/CodeEditor";
import { Button } from "@/components/ui/button";
import { CommandPalette } from "@/components/ui/command-palette";
import {
	ResizableHandle,
	ResizablePanel,
	ResizablePanelGroup,
} from "@/components/ui/resizable";
import { Toaster } from "@/components/ui/sonner";
import {
	AgentFilterForm,
	type AgentFilterValues,
} from "@/features/agents/AgentFilterForm";
import { AgentsTable } from "@/features/agents/AgentsTable";
import { listAgents } from "@/lib/agents";

const SAMPLE_CONFIG = `{
  "mcpServers": {
    "filesystem": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-filesystem", "/workspace"]
    }
  }
}`;

const SAMPLE_MODIFIED = `{
  "mcpServers": {
    "filesystem": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-filesystem", "/home/dev/projects"]
    },
    "github": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-github"],
      "env": {
        "GITHUB_PERSONAL_ACCESS_TOKEN": "\${GITHUB_TOKEN}"
      }
    }
  }
}`;

function RootLayout() {
	const navigate = useNavigate();
	const queryClient = useQueryClient();

	function handleRefreshAgents() {
		void queryClient.invalidateQueries({ queryKey: ["agents"] });
		toast.success("Agents status refreshed");
	}

	return (
		<div className="min-h-screen bg-background text-foreground">
			<header className="border-b border-border bg-card/60 px-6 py-4">
				<div className="mx-auto flex max-w-6xl flex-wrap items-center justify-between gap-4">
					<div className="space-y-1">
						<h1 className="text-2xl font-bold tracking-tight text-white">
							Agents Manager
						</h1>
						<p className="text-xs text-muted-foreground">
							Inspect detected coding agents and manage local configurations
							from a unified workspace.
						</p>
					</div>

					<nav
						aria-label="Primary navigation"
						className="flex items-center gap-2"
					>
						<Link
							to="/"
							className="inline-flex items-center gap-1.5 rounded-md px-3 py-1.5 text-sm font-medium text-muted-foreground transition-colors hover:bg-accent hover:text-foreground [&.active]:bg-secondary [&.active]:text-foreground"
						>
							<LayoutGrid className="size-4" aria-hidden="true" />
							<span>Overview</span>
						</Link>
						<Link
							to="/editor"
							className="inline-flex items-center gap-1.5 rounded-md px-3 py-1.5 text-sm font-medium text-muted-foreground transition-colors hover:bg-accent hover:text-foreground [&.active]:bg-secondary [&.active]:text-foreground"
						>
							<FileCode2 className="size-4" aria-hidden="true" />
							<span>Editor &amp; Diff</span>
						</Link>
						<Button variant="outline" size="sm" onClick={handleRefreshAgents}>
							<RefreshCw className="size-3.5" aria-hidden="true" />
							<span>Refresh</span>
						</Button>
						<span className="hidden items-center gap-1 rounded border border-border px-2 py-1 text-xs text-muted-foreground sm:inline-flex">
							<Command className="size-3" aria-hidden="true" />
							<span>Ctrl+K</span>
						</span>
					</nav>
				</div>
			</header>

			<main className="mx-auto max-w-6xl px-6 py-8">
				<Outlet />
			</main>

			<CommandPalette
				onNavigate={(path) => {
					void navigate({ to: path });
				}}
				onRefreshAgents={handleRefreshAgents}
			/>
			<Toaster />
		</div>
	);
}

function OverviewPage() {
	const { data, isLoading, isError, error } = useQuery({
		queryKey: ["agents"],
		queryFn: listAgents,
	});
	const [filter, setFilter] = useState<AgentFilterValues>({
		query: "",
		status: "all",
	});

	const filteredAgents = useMemo(() => {
		const agents = data ?? [];
		const needle = filter.query.trim().toLowerCase();
		return agents.filter((agent) => {
			if (filter.status === "detected" && !agent.detected) {
				return false;
			}
			if (filter.status === "missing" && agent.detected) {
				return false;
			}
			if (!needle) {
				return true;
			}
			return (
				agent.name.toLowerCase().includes(needle) ||
				agent.kind.toLowerCase().includes(needle) ||
				(agent.configPath ?? "").toLowerCase().includes(needle)
			);
		});
	}, [data, filter]);

	if (isLoading) {
		return (
			<p role="status" className="text-sm text-muted-foreground">
				Loading agents...
			</p>
		);
	}

	if (isError) {
		return (
			<p
				role="alert"
				className="rounded-lg border border-rose-500/30 bg-rose-500/10 p-4 text-sm text-rose-200"
			>
				Failed to load agents:{" "}
				{error instanceof Error ? error.message : "Unknown error"}
			</p>
		);
	}

	const agents = data ?? [];

	return (
		<div className="space-y-8">
			<section
				aria-label="Supported agents"
				className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3"
			>
				{agents.map((agent) => (
					<AgentCard key={agent.kind} agent={agent} />
				))}
			</section>

			<section aria-label="Agent inventory" className="space-y-4">
				<AgentFilterForm initialValues={filter} onFilterChange={setFilter} />
				<AgentsTable agents={filteredAgents} />
			</section>
		</div>
	);
}

function EditorPage() {
	const [draft, setDraft] = useState(SAMPLE_MODIFIED);

	return (
		<div className="space-y-4">
			<div className="flex items-center justify-between">
				<div>
					<h2 className="text-lg font-semibold text-white">
						Config &amp; Diff Preview
					</h2>
					<p className="text-xs text-muted-foreground">
						Edit shared configuration and inspect changes before applying sync.
					</p>
				</div>
				<Button
					size="sm"
					onClick={() => toast.info("Preview ready — backend sync comes next")}
				>
					Preview Sync
				</Button>
			</div>

			<div className="h-[460px] rounded-xl border border-border bg-card p-3">
				<ResizablePanelGroup>
					<ResizablePanel defaultSize="45%" minSize="25%">
						<div className="flex h-full flex-col gap-2 pr-2">
							<span className="text-xs font-medium text-muted-foreground">
								Draft JSON
							</span>
							<CodeEditor
								value={draft}
								language="json"
								onChange={setDraft}
								className="flex-1"
							/>
						</div>
					</ResizablePanel>
					<ResizableHandle />
					<ResizablePanel defaultSize="55%" minSize="30%">
						<div className="flex h-full flex-col gap-2 pl-2">
							<span className="text-xs font-medium text-muted-foreground">
								Side-by-side Diff
							</span>
							<DiffViewer
								original={SAMPLE_CONFIG}
								modified={draft}
								language="json"
								className="flex-1"
							/>
						</div>
					</ResizablePanel>
				</ResizablePanelGroup>
			</div>
		</div>
	);
}

const rootRoute = createRootRoute({
	component: RootLayout,
});

const indexRoute = createRoute({
	getParentRoute: () => rootRoute,
	path: "/",
	component: OverviewPage,
});

const editorRoute = createRoute({
	getParentRoute: () => rootRoute,
	path: "/editor",
	component: EditorPage,
});

const routeTree = rootRoute.addChildren([indexRoute, editorRoute]);

export function createAppRouter(history?: RouterHistory) {
	return createRouter({
		routeTree,
		history: history ?? createHashHistory(),
	});
}
