import type { Meta, StoryObj } from "@storybook/react-vite";
import { SyncPreviewList } from "./SyncPreviewList";

const meta: Meta<typeof SyncPreviewList> = {
	title: "Features/Sync preview",
	component: SyncPreviewList,
};
export default meta;
export const Masked: StoryObj<typeof SyncPreviewList> = {
	args: {
		items: [
			{
				targetPath: "/example/config.json",
				agent: "codex",
				scope: "global",
				strategy: "merge",
				currentContent: '{"token":"old-secret"}',
				proposedContent: '{"token":"new-secret"}',
				currentSecretSpans: [{ startByte: 10, endByte: 20, kind: "token" }],
				proposedSecretSpans: [{ startByte: 10, endByte: 20, kind: "token" }],
				expectedSha256: "synthetic-hash",
				changed: true,
				warnings: [],
			},
		],
	},
};
