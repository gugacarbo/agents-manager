import { useQueryClient } from "@tanstack/react-query";
import {
	createHashHistory,
	createRootRoute,
	createRoute,
	createRouter,
	Link,
	Outlet,
	type RouterHistory,
	redirect,
	useLocation,
	useNavigate,
} from "@tanstack/react-router";
import {
	ChevronRight,
	FileCode2,
	FileText,
	FolderOpen,
	LayoutGrid,
	Library,
	Monitor,
	Terminal,
} from "lucide-react";
import {
	RouteErrorFallback,
	RouteNotFoundFallback,
} from "@/components/global-route-fallback";
import { ThemeProvider } from "@/components/theme-provider";
import { ThemeToggle } from "@/components/theme-toggle";
import { Button } from "@/components/ui/button";
import { CommandPalette } from "@/components/ui/command-palette";
import { Toaster } from "@/components/ui/sonner";
import { AgentsPage, agentsQueryKey } from "@/features/agents";
import { AgentConfigsPage } from "@/features/configs";
import { DiffDemoPage } from "@/features/diff";
import { LibraryPage } from "@/features/library";
import { ProjectsPage } from "@/features/projects";
import { RulesPage } from "@/features/rules";
import { isTauriRuntime } from "@/lib/agents";
import { agentKindSchema } from "@/lib/schemas";

const navLink =
	"flex min-h-10 items-center gap-2.5 rounded-md px-3 text-sm font-medium text-muted-foreground transition-colors hover:bg-accent hover:text-foreground data-[section-active=true]:bg-selection data-[section-active=true]:text-selection-foreground";

const navigation = [
	{ to: "/", label: "Agents", icon: LayoutGrid },
	{ to: "/projects", label: "Projects", icon: FolderOpen },
	{ to: "/library", label: "Library", icon: Library },
] as const;

function RootLayout() {
	const navigate = useNavigate();
	const queryClient = useQueryClient();
	const pathname = useLocation({ select: (location) => location.pathname });
	const desktop = isTauriRuntime();
	const section = pathname.split("/")[1];
	const activeSection = section === "agents" ? "/" : `/${section}`;
	const tools = desktop
		? ({ to: "/rules", label: "Rules", icon: FileText } as const)
		: ({ to: "/demo", label: "Diff demo", icon: FileCode2 } as const);
	const links = [...navigation, tools];
	const currentSection = links.find(({ to }) => to === activeSection);
	const isConfiguration =
		section === "agents" || pathname.startsWith("/projects/");

	return (
		<ThemeProvider>
			<div className="workspace-shell min-h-screen md:grid">
				<Button
					type="button"
					className="sr-only z-20 rounded-md bg-primary px-4 py-2 text-sm font-medium text-primary-foreground focus:not-sr-only focus:fixed focus:top-3 focus:left-3"
					onClick={() => {
						document.getElementById("main-content")?.focus();
					}}
				>
					Skip to content
				</Button>
				<aside className="flex flex-col border-b border-border bg-sidebar md:sticky md:top-0 md:h-screen md:overflow-y-auto md:border-r md:border-b-0">
					<div className="flex min-h-16 items-center gap-3 px-4 md:px-5">
						<span className="grid size-8 shrink-0 place-items-center rounded-md bg-primary text-primary-foreground">
							<Terminal className="size-4.5" aria-hidden="true" />
						</span>
						<span className="text-sm font-semibold tracking-tight">
							Agents Manager
						</span>
					</div>
					<nav
						aria-label="Primary navigation"
						className="flex flex-wrap gap-1 px-3 pb-3 md:flex-col md:px-4 md:pt-5"
					>
						<p className="mb-2 hidden px-3 text-xs font-medium text-muted-foreground md:block">
							Workspace
						</p>
						{links.map(({ to, label, icon: Icon }) => (
							<Link
								key={to}
								to={to}
								activeOptions={{ exact: true }}
								data-section-active={to === activeSection}
								className={navLink}
							>
								<Icon className="size-4 shrink-0" aria-hidden="true" />
								{label}
							</Link>
						))}
					</nav>
					<div className="mt-auto hidden items-center gap-3 border-t border-border px-5 py-4 md:flex">
						<Monitor
							className="size-4 text-muted-foreground"
							aria-hidden="true"
						/>
						<span className="text-xs text-muted-foreground">
							{desktop ? "Local desktop" : "Browser preview"}
						</span>
					</div>
				</aside>
				<div className="min-w-0">
					<header className="flex min-h-16 flex-wrap items-center justify-between gap-2 border-b border-border px-4 py-2 sm:px-8 lg:px-12">
						<div className="flex min-w-0 flex-wrap items-center gap-2 text-xs text-muted-foreground sm:text-sm">
							<span>Workspace</span>
							{currentSection && (
								<>
									<ChevronRight className="size-3.5" aria-hidden="true" />
									<span
										className={
											isConfiguration
												? undefined
												: "font-medium text-foreground"
										}
									>
										{currentSection.label}
									</span>
								</>
							)}
							{isConfiguration && (
								<>
									<ChevronRight className="size-3.5" aria-hidden="true" />
									<span className="font-medium text-foreground">
										Configuration
									</span>
								</>
							)}
						</div>
						<div className="flex items-center gap-2">
							<ThemeToggle />
							<CommandPalette
								showDemo={!desktop}
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
					</header>
					<main
						id="main-content"
						tabIndex={-1}
						className="mx-auto max-w-7xl px-4 py-7 focus-visible:-outline-offset-2 sm:px-8 sm:py-9 lg:px-12 lg:py-11"
					>
						<Outlet />
					</main>
				</div>
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
	beforeLoad: () => {
		throw redirect({ to: "/", replace: true });
	},
});
const demoRoute = createRoute({
	getParentRoute: () => rootRoute,
	path: "/demo",
	component: DiffDemoPage,
});
const rulesRoute = createRoute({
	getParentRoute: () => rootRoute,
	path: "/rules",
	component: RulesPage,
});
const agentRoute = createRoute({
	getParentRoute: () => rootRoute,
	path: "/agents/$agentKind",
	component: AgentRoute,
});
function AgentRoute() {
	const { agentKind } = agentRoute.useParams();
	const kind = agentKindSchema.safeParse(agentKind);
	return kind.success ? (
		<AgentConfigsPage agentKind={kind.data} />
	) : (
		<DefaultNotFound />
	);
}
const projectsRoute = createRoute({
	getParentRoute: () => rootRoute,
	path: "/projects",
	component: () => <ProjectsPage preview={!isTauriRuntime()} />,
});
const projectAgentRoute = createRoute({
	getParentRoute: () => rootRoute,
	path: "/projects/$projectId/agents/$agentKind",
	component: ProjectAgentRoute,
});
function ProjectAgentRoute() {
	const { projectId, agentKind } = projectAgentRoute.useParams();
	const kind = agentKindSchema.safeParse(agentKind);
	return kind.success ? (
		<AgentConfigsPage agentKind={kind.data} projectId={projectId} />
	) : (
		<DefaultNotFound />
	);
}
const libraryRoute = createRoute({
	getParentRoute: () => rootRoute,
	path: "/library",
	component: LibraryPage,
});
const routeTree = rootRoute.addChildren([
	indexRoute,
	editorRoute,
	demoRoute,
	rulesRoute,
	agentRoute,
	projectsRoute,
	projectAgentRoute,
	libraryRoute,
]);

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
