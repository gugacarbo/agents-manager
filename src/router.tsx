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
	FileCode2,
	FileText,
	FolderOpen,
	LayoutGrid,
	Library,
	Monitor,
	Users,
} from "lucide-react";
import { AppPreferencesMenu } from "@/components/app-preferences-menu";
import { BrandMark } from "@/components/brand-mark";
import { DesktopTitleBar } from "@/components/desktop-title-bar";
import {
	RouteErrorFallback,
	RouteNotFoundFallback,
} from "@/components/global-route-fallback";
import { ThemeProvider } from "@/components/theme-provider";
import {
	Breadcrumb,
	BreadcrumbItem,
	BreadcrumbList,
	BreadcrumbPage,
	BreadcrumbSeparator,
} from "@/components/ui/breadcrumb";
import { Button } from "@/components/ui/button";
import { CommandPalette } from "@/components/ui/command-palette";
import {
	Sidebar,
	SidebarContent,
	SidebarFooter,
	SidebarGroup,
	SidebarGroupLabel,
	SidebarHeader,
	SidebarInset,
	SidebarMenu,
	SidebarMenuItem,
	SidebarMenuLabel,
	SidebarProvider,
	SidebarTrigger,
} from "@/components/ui/sidebar";
import { Toaster } from "@/components/ui/sonner";
import {
	Tooltip,
	TooltipContent,
	TooltipProvider,
	TooltipTrigger,
} from "@/components/ui/tooltip";
import { AgentsPage, agentsQueryKey } from "@/features/agents";
import { ClaudeAccountProfiles } from "@/features/claude-accounts";
import { AgentConfigsPage } from "@/features/configs";
import { DiffDemoPage } from "@/features/diff";
import { EnvironmentSwitcher } from "@/features/environments";
import { LibraryPage } from "@/features/library";
import { ProjectsPage } from "@/features/projects";
import { RulesPage } from "@/features/rules";
import { isTauriRuntime } from "@/lib/agents";
import { agentKindSchema } from "@/lib/schemas";

const navLink =
	"flex min-h-10 items-center gap-2.5 rounded-md px-2.5 text-sm font-medium text-muted-foreground transition-colors hover:bg-accent hover:text-foreground data-[section-active=true]:bg-selection data-[section-active=true]:text-selection-foreground group-data-[state=collapsed]/sidebar:md:min-h-9";

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
	const isClaudeAccounts = pathname === "/accounts/claude";
	const activeSection = isClaudeAccounts
		? "/accounts/claude"
		: section === "agents"
			? "/"
			: `/${section}`;
	const links = desktop
		? ([
				...navigation,
				{ to: "/rules", label: "Rules", icon: FileText },
				{ to: "/accounts/claude", label: "Claude Code accounts", icon: Users },
			] as const)
		: ([
				...navigation,
				{ to: "/demo", label: "Diff demo", icon: FileCode2 },
			] as const);
	const currentSection = links.find(({ to }) => to === activeSection);
	const isConfiguration =
		section === "agents" || pathname.startsWith("/projects/");

	const toolbarActions = (
		<>
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
			<AppPreferencesMenu showTrayOption={desktop} />
		</>
	);

	const breadcrumb = (
		<Breadcrumb>
			<BreadcrumbList className="text-xs sm:gap-1.5">
				<BreadcrumbItem>Workspace</BreadcrumbItem>
				{currentSection && (
					<>
						<BreadcrumbSeparator />
						<BreadcrumbItem>
							{isConfiguration ? (
								<span>{currentSection.label}</span>
							) : (
								<BreadcrumbPage className="font-medium">
									{currentSection.label}
								</BreadcrumbPage>
							)}
						</BreadcrumbItem>
					</>
				)}
				{isConfiguration && (
					<>
						<BreadcrumbSeparator />
						<BreadcrumbItem>
							<BreadcrumbPage className="font-medium">
								Configuration
							</BreadcrumbPage>
						</BreadcrumbItem>
					</>
				)}
			</BreadcrumbList>
		</Breadcrumb>
	);

	return (
		<ThemeProvider>
			<TooltipProvider>
				<SidebarProvider
					className="workspace-shell data-[desktop=true]:h-screen data-[desktop=true]:overflow-hidden"
					data-desktop={desktop}
				>
					<Button
						type="button"
						className="sr-only z-20 rounded-md bg-primary px-4 py-2 text-sm font-medium text-primary-foreground focus:not-sr-only focus:fixed focus:top-3 focus:left-3"
						onClick={() => {
							document.getElementById("main-content")?.focus();
						}}
					>
						Skip to content
					</Button>
					<Sidebar>
						<SidebarHeader data-tauri-drag-region={desktop ? "" : undefined}>
							<BrandMark className="pointer-events-none size-8 shrink-0" />
							<SidebarMenuLabel className="pointer-events-none text-sm font-semibold tracking-tight">
								Agents Manager
							</SidebarMenuLabel>
						</SidebarHeader>
						<SidebarContent aria-label="Primary navigation">
							<SidebarGroup>
								<SidebarGroupLabel>Workspace</SidebarGroupLabel>
								<SidebarMenu>
									{links.map(({ to, label, icon: Icon }) => (
										<SidebarMenuItem key={to}>
											<Tooltip>
												<TooltipTrigger asChild>
													<Link
														to={to}
														activeOptions={{ exact: true }}
														data-section-active={to === activeSection}
														className={navLink}
													>
														<Icon
															className="size-4 shrink-0"
															aria-hidden="true"
														/>
														<SidebarMenuLabel>{label}</SidebarMenuLabel>
													</Link>
												</TooltipTrigger>
												<TooltipContent side="right">{label}</TooltipContent>
											</Tooltip>
										</SidebarMenuItem>
									))}
								</SidebarMenu>
							</SidebarGroup>
						</SidebarContent>
						<SidebarFooter>
							<Monitor
								className="size-4 shrink-0 text-muted-foreground"
								aria-hidden="true"
							/>
							<SidebarMenuLabel className="text-xs text-muted-foreground">
								{desktop ? "Local desktop" : "Browser preview"}
							</SidebarMenuLabel>
						</SidebarFooter>
					</Sidebar>
					<SidebarInset>
						{desktop ? (
							<DesktopTitleBar
								leading={<SidebarTrigger />}
								breadcrumb={breadcrumb}
								actions={toolbarActions}
								offsetSidebar
							/>
						) : null}
						<div className="min-h-0 flex-1 overflow-y-auto">
							{desktop ? null : (
								<header className="flex min-h-16 flex-wrap items-center justify-between gap-2 border-b border-border px-4 py-2 sm:px-8 lg:px-12">
									<div className="flex min-w-0 flex-wrap items-center gap-2 text-xs text-muted-foreground sm:text-sm">
										<SidebarTrigger className="mr-1" />
										{breadcrumb}
									</div>
									<div className="flex items-center gap-2">
										{toolbarActions}
									</div>
								</header>
							)}
							<main
								id="main-content"
								tabIndex={-1}
								className="mx-auto max-w-7xl px-4 py-7 focus-visible:-outline-offset-2 sm:px-8 sm:py-9 lg:px-12 lg:py-11"
							>
								{isClaudeAccounts ? (
									<Outlet />
								) : (
									<EnvironmentSwitcher>
										<Outlet />
									</EnvironmentSwitcher>
								)}
							</main>
						</div>
					</SidebarInset>
				</SidebarProvider>
			</TooltipProvider>
			<Toaster />
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
const claudeAccountsRoute = createRoute({
	getParentRoute: () => rootRoute,
	path: "/accounts/claude",
	beforeLoad: () => {
		if (!isTauriRuntime()) throw redirect({ to: "/", replace: true });
	},
	component: ClaudeAccountProfiles,
});
const routeTree = rootRoute.addChildren([
	claudeAccountsRoute,
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

if (import.meta.hot) {
	import.meta.hot.accept(() => {
		window.location.reload();
	});
}
