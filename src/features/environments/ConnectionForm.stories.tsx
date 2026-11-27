import type { Meta, StoryObj } from "@storybook/react-vite";
import { ConnectionForm } from "./ConnectionForm";

const meta: Meta<typeof ConnectionForm> = {
	title: "Features/SSH environment",
	component: ConnectionForm,
};
export default meta;
export const NewEnvironment: StoryObj<typeof ConnectionForm> = {
	args: {
		initial: null,
		onClose: () => {},
		onSaved: () => {},
		onChange: () => {},
	},
};
export const ManualEnvironment: StoryObj<typeof ConnectionForm> = {
	args: {
		initial: {
			id: "example",
			name: "Work server",
			connection: {
				mode: "manual",
				host: "host.example",
				user: "developer",
				port: 22,
			},
		},
		onClose: () => {},
		onSaved: () => {},
		onChange: () => {},
	},
};
