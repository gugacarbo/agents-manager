import type { Meta, StoryObj } from "@storybook/react-vite";
import { Button } from "@/components/ui/button";
import { EmptyState } from "./EmptyState";

const meta: Meta<typeof EmptyState> = {
	title: "Patterns/EmptyState",
	component: EmptyState,
};

export default meta;
type Story = StoryObj<typeof EmptyState>;

export const Default: Story = {
	args: {
		title: "No agents match your filters.",
		description: "Adjust your search query or status filter to view agents.",
		action: (
			<Button variant="outline" size="sm">
				Clear filters
			</Button>
		),
	},
};
