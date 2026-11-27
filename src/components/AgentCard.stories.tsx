import type { Meta, StoryObj } from "@storybook/react-vite";
import { AgentCard } from "./AgentCard";

const meta: Meta<typeof AgentCard> = {
	title: "Components/AgentCard",
	component: AgentCard,
};

export default meta;
type Story = StoryObj<typeof AgentCard>;

export const Detected: Story = {
	args: {
		agent: {
			kind: "claude-code",
			name: "Claude Code",
			detected: true,
			configPath: "/home/dev/.claude",
		},
	},
};

export const NotDetected: Story = {
	args: {
		agent: {
			kind: "codex",
			name: "Codex",
			detected: false,
			configPath: null,
		},
	},
};
