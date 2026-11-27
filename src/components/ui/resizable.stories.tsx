import type { Meta, StoryObj } from "@storybook/react-vite";
import {
	ResizableHandle,
	ResizablePanel,
	ResizablePanelGroup,
} from "./resizable";

const meta: Meta<typeof ResizablePanelGroup> = {
	title: "UI/Resizable",
	component: ResizablePanelGroup,
};

export default meta;
type Story = StoryObj<typeof ResizablePanelGroup>;

export const SplitPane: Story = {
	render: () => (
		<div className="h-64 w-full rounded-lg border border-border bg-card">
			<ResizablePanelGroup>
				<ResizablePanel defaultSize="45%" minSize="25%">
					<div className="p-4 text-sm text-muted-foreground">Left pane</div>
				</ResizablePanel>
				<ResizableHandle />
				<ResizablePanel defaultSize="55%" minSize="25%">
					<div className="p-4 text-sm text-muted-foreground">Right pane</div>
				</ResizablePanel>
			</ResizablePanelGroup>
		</div>
	),
};
