import type { Meta, StoryObj } from "@storybook/react-vite";
import { Button } from "@/components/ui/button";
import { ErrorState } from "./ErrorState";

const meta: Meta<typeof ErrorState> = {
	title: "Patterns/ErrorState",
	component: ErrorState,
};

export default meta;
type Story = StoryObj<typeof ErrorState>;

export const Default: Story = {
	args: {
		title: "Could not check local configuration folders.",
		message: "Not found: user home directory",
	},
};

export const WithRetryAction: Story = {
	args: {
		title: "Failed to load agents",
		message: "I/O error: permission denied while reading configuration path",
		action: (
			<Button variant="outline" size="sm">
				Try again
			</Button>
		),
	},
};
