import { useQueryClient } from "@tanstack/react-query";
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
import { FileCode2, LayoutGrid, Terminal } from "lucide-react";
import {
	RouteErrorFallback,
	RouteNotFoundFallback,
} from "@/components/global-route-fallback";
import { ThemeProvider } from "@/components/theme-provider";
import { ThemeToggle } from "@/components/theme-toggle";
import { CommandPalette } from "@/components/ui/command-palette";
import { Toaster } from "@/components/ui/sonner";
import { AgentsPage, agentsQueryKey } from "@/features/agents";
import { DiffDemoPage } from "@/features/diff";

const navLink =
	"inline-flex min-h-9 items-center gap-2 rounded-md px-3 text-sm font-medium text-muted-foreground hover:bg-accent hover:text-foreground aria-[current=page]:bg-selection aria-[current=page]:text-selection-foreground";

function RootLayout() {
	const navigate = useNavigate();
	const queryClient = useQueryClient();

	return (
		<ThemeProvider>
			<div className="min-h-screen">
				<header className="border-b border-border bg-card/60 px-4 sm:px-6">
					<div className="mx-auto flex min-h-16 max-w-6xl flex-wrap items-center gap-x-6 gap-y-2 py-2.5">
						<h1 className="flex items-center gap-2.5 text-base font-semibold tracking-tight">
							<span className="grid size-8 place-items-center rounded-md border border-border bg-card text-ring">
								<Terminal className="size-4" aria-hidden="true" />
							</span>
							Agents Manager
						</h1>
						<nav
							aria-label="Primary navigation"
							className="flex items-center gap-1"
						>
							<Link to="/" activeOptions={{ exact: true }} className={navLink}>
								<LayoutGrid className="size-4" aria-hidden="true" />
								Agents
							</Link>
							<Link to="/editor" className={navLink}>
								<FileCode2 className="size-4" aria-hidden="true" />
								Diff demo
							</Link>
						</nav>
						<div className="ml-auto flex items-center gap-2">
							<ThemeToggle />
							<CommandPalette
								onNavigate={(path) => {
									void navigate({ to: path });
								}}
								onRefreshAgents={() => {
									void queryClient.invalidateQueries(
										{ queryKey: agentsQueryKey },
										{ cancelRefetch: false },
									);
									void navigate({ to: "/" });
								}}
							/>
						</div>
					</div>
				</header>
				<main className="mx-auto max-w-6xl px-4 py-7 sm:px-6 sm:py-8">
					<Outlet />
				</main>
				<Toaster />
			</div>
		</ThemeProvider>
	);
}

const rootRoute = createRootRoute({ component: RootLayout });
const indexRoute = createRoute({
	getParentRoute: () => rootRoute,
	path: "/",
	component: AgentsPage,
});
const editorRoute = createRoute({
	getParentRoute: () => rootRoute,
	path: "/editor",
	component: DiffDemoPage,
});
const routeTree = rootRoute.addChildren([indexRoute, editorRoute]);

function DefaultNotFound() {
	const navigate = useNavigate();
	return (
		<RouteNotFoundFallback
			onGoHome={() => {
				void navigate({ to: "/" });
			}}
		/>
	);
}

export function createAppRouter(history?: RouterHistory) {
	return createRouter({
		routeTree,
		history: history ?? createHashHistory(),
		defaultNotFoundComponent: DefaultNotFound,
		defaultErrorComponent: RouteErrorFallback,
	});
}
