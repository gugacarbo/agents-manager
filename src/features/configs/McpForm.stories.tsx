import type { Meta, StoryObj } from "@storybook/react-vite";
import { useState } from "react";
import type { Server } from "@/lib/structured";
import { McpForm } from "./McpForm";

const meta: Meta<typeof McpForm> = {
	title: "Features/MCP form",
	component: McpForm,
};
export default meta;
export const WithServer: StoryObj<typeof McpForm> = {
	render: () => <Example />,
};
function Example() {
	const [servers, setServers] = useState<Server[]>([
		{
			name: "example",
			transport: "stdio",
			command: "node",
			args: ["server.js"],
			enabled: true,
			env: [
				{
					key: "TOKEN",
					value: "synthetic-secret",
					isSecret: true,
					isEnvRef: false,
				},
			],
		},
	]);
	return (
		<McpForm
			servers={servers}
			existingNames={["example"]}
			onChange={setServers}
		/>
	);
}
