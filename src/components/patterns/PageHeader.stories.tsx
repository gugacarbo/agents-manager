import type { Meta, StoryObj } from "@storybook/react-vite";
import { Button } from "@/components/ui/button";
import { PageHeader } from "./PageHeader";

const meta: Meta<typeof PageHeader> = {
	title: "Patterns/PageHeader",
	component: PageHeader,
};

export default meta;
type Story = StoryObj<typeof PageHeader>;

export const Default: Story = {
	args: {
		title: "Local agents",
		description: "Find configuration folders for supported coding agents.",
	},
};

export const WithActions: Story = {
	args: {
		title: "Config & Diff Preview",
		description:
			"Edit shared configuration and inspect changes before applying sync.",
		actions: <Button size="sm">Preview Sync</Button>,
	},
};
