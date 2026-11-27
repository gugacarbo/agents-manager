import type { Meta, StoryObj } from "@storybook/react-vite";
import { CodeEditor, DiffViewer } from "./CodeEditor";

const meta: Meta<typeof CodeEditor> = {
	title: "Components/CodeEditor",
	component: CodeEditor,
};

export default meta;
type Story = StoryObj<typeof CodeEditor>;

export const JsonConfig: Story = {
	args: {
		language: "json",
		value: '{\n  "mcpServers": {\n    "github": { "command": "npx" }\n  }\n}',
	},
};

export const ConfigDiff: StoryObj<typeof DiffViewer> = {
	render: () => (
		<DiffViewer
			language="json"
			original='{\n  "model": "sonnet"\n}'
			modified='{\n  "model": "sonnet",\n  "sandbox": true\n}'
		/>
	),
};

export const MaskedSecret: Story = {
	args: {
		language: "json",
		value: '{"token":"example-secret"}',
		secretSpans: [{ startByte: 10, endByte: 24, kind: "token" }],
	},
};
