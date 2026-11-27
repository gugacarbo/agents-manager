import type { Meta, StoryObj } from "@storybook/react-vite";
import { FolderOpen, LayoutGrid, Library, Monitor } from "lucide-react";
import { BrandMark } from "@/components/brand-mark";
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
} from "./sidebar";

const meta: Meta<typeof SidebarProvider> = {
	title: "UI/Sidebar",
	component: SidebarProvider,
	parameters: { layout: "fullscreen" },
};

export default meta;
type Story = StoryObj<typeof SidebarProvider>;

function SidebarDemo({ defaultOpen }: { defaultOpen: boolean }) {
	return (
		<SidebarProvider defaultOpen={defaultOpen}>
			<Sidebar>
				<SidebarHeader>
					<BrandMark className="size-8 shrink-0" />
					<SidebarMenuLabel className="text-sm font-semibold tracking-tight">
						Agents Manager
					</SidebarMenuLabel>
				</SidebarHeader>
				<SidebarContent aria-label="Primary navigation">
					<SidebarGroup>
						<SidebarGroupLabel>Workspace</SidebarGroupLabel>
						<SidebarMenu>
							{[
								{ label: "Agents", icon: LayoutGrid, active: true },
								{ label: "Projects", icon: FolderOpen, active: false },
								{ label: "Library", icon: Library, active: false },
							].map(({ label, icon: Icon, active }) => (
								<SidebarMenuItem key={label}>
									<span
										data-section-active={active}
										className="flex min-h-10 items-center gap-2.5 rounded-md px-3 text-sm font-medium text-muted-foreground data-[section-active=true]:bg-selection data-[section-active=true]:text-selection-foreground"
									>
										<Icon className="size-4 shrink-0" aria-hidden="true" />
										<SidebarMenuLabel>{label}</SidebarMenuLabel>
									</span>
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
						Local desktop
					</SidebarMenuLabel>
				</SidebarFooter>
			</Sidebar>
			<SidebarInset>
				<div className="flex h-12 items-center gap-2 border-b border-border px-4">
					<SidebarTrigger />
					<span className="text-sm text-muted-foreground">Workspace</span>
				</div>
				<div className="p-6 text-sm text-muted-foreground">Main content</div>
			</SidebarInset>
		</SidebarProvider>
	);
}

export const Expanded: Story = {
	render: () => <SidebarDemo defaultOpen />,
};

export const Collapsed: Story = {
	render: () => <SidebarDemo defaultOpen={false} />,
};
