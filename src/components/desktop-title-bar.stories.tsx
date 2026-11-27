import type { Meta, StoryObj } from "@storybook/react-vite";
import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import { DesktopTitleBar } from "./desktop-title-bar";

const meta = {
	title: "App/DesktopTitleBar",
	component: DesktopTitleBar,
	parameters: { layout: "fullscreen" },
	beforeEach: () => {
		mockWindows("main");
		mockIPC(() => null);
		return () => {
			delete (window as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;
		};
	},
	decorators: [
		(Story) => (
			<div className="workspace-shell" data-desktop="true">
				<Story />
			</div>
		),
	],
} satisfies Meta<typeof DesktopTitleBar>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {};
