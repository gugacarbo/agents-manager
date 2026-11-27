import {
	act,
	cleanup,
	fireEvent,
	render,
	screen,
	waitFor,
} from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { ConfigEditor } from "./ConfigEditor";

const { state } = vi.hoisted(() => ({
	state: {
		changed: (_hash: string | null) => {},
		reads: 0,
		save: vi.fn(),
		preview: vi.fn(),
		saveStructured: vi.fn(),
	},
}));
vi.mock("@/components/CodeEditor", () => ({
	CodeEditor: ({
		value,
		onChange,
	}: {
		value: string;
		onChange: (value: string, masked: string) => void;
	}) => (
		<textarea
			aria-label="Configuration editor"
			value={value}
			onChange={(event) => onChange(event.target.value, event.target.value)}
		/>
	),
	DiffViewer: ({
		original,
		modified,
	}: {
		original: string;
		modified: string;
	}) => (
		<div>
			{original} → {modified}
		</div>
	),
}));
vi.mock("@/lib/configs", () => ({
	configs: {
		watch: async (_path: string, changed: (hash: string | null) => void) => {
			state.changed = changed;
			return () => {};
		},
		read: async () => {
			state.reads++;
			return {
				entry: {
					agent: "codex",
					category: "settings",
					label: "Config",
					path: "/test",
					format: "json",
					exists: true,
				},
				content: '{"disk":true}',
				sha256: "latest",
				secretSpans: [],
			};
		},
		save: state.save,
		message: () => "Error",
	},
	structuredConfigs: { preview: state.preview, save: state.saveStructured },
	isExternalConflict: (error: unknown) => error === "ExternalConflict",
}));
afterEach(cleanup);

describe("external changes", () => {
	it("requires reviewing the latest disk version and saves against its hash", async () => {
		state.save.mockResolvedValue({ sha256: "saved", backupPath: "/backup" });
		render(
			<ConfigEditor
				initial={{
					entry: {
						agent: "codex",
						category: "settings",
						label: "Config",
						path: "/test",
						format: "json",
						exists: true,
					},
					content: '{"old":true,"token":"hidden-value"}',
					sha256: "old",
					secretSpans: [{ startByte: 21, endByte: 33, kind: "token" }],
				}}
			/>,
		);
		expect(screen.getByText(/••••••/)).not.toHaveTextContent("hidden-value");
		fireEvent.change(
			screen.getByRole("textbox", { name: "Configuration editor" }),
			{ target: { value: '{"draft":true}' } },
		);
		await act(async () => state.changed("changed"));
		expect(screen.getByRole("button", { name: "Save" })).toBeDisabled();
		fireEvent.click(
			screen.getByRole("button", { name: "Overwrite (review diff)" }),
		);
		expect(
			await screen.findByText(/Review the diff against/),
		).toBeInTheDocument();
		expect(screen.getByText(/"disk":true/)).toHaveTextContent('{"draft":true}');
		fireEvent.click(screen.getByRole("button", { name: "Confirm overwrite" }));
		await waitFor(() =>
			expect(state.save).toHaveBeenCalledWith(
				"/test",
				'{"draft":true}',
				"latest",
			),
		);
	});
});

describe("structured editing", () => {
	it("preserves a valid raw draft and requires preview before structured save", async () => {
		state.preview.mockResolvedValue({
			content: '{"old":2,"other":true}',
			secretSpans: [],
		});
		render(
			<ConfigEditor
				initial={{
					entry: {
						agent: "codex",
						category: "settings",
						label: "Config",
						path: "/test",
						format: "json",
						exists: true,
					},
					content: '{"old":1}',
					sha256: "old",
					secretSpans: [],
				}}
			/>,
		);
		fireEvent.change(
			screen.getByRole("textbox", { name: "Configuration editor" }),
			{ target: { value: '{"old":1,"other":true}' } },
		);
		fireEvent.click(screen.getByRole("button", { name: "Structured" }));
		fireEvent.change(screen.getByLabelText("old"), { target: { value: "2" } });
		expect(screen.getByRole("button", { name: "Save" })).toBeDisabled();
		fireEvent.click(screen.getByRole("button", { name: "Preview changes" }));
		await waitFor(() =>
			expect(state.preview).toHaveBeenCalledWith(
				expect.objectContaining({
					path: "/test",
					expectedSha256: "old",
					settingsPatch: { old: 2 },
				}),
				'{"old":1,"other":true}',
			),
		);
		expect(await screen.findByText(/"old":2/)).toBeInTheDocument();
		expect(screen.getByRole("button", { name: "Save" })).toBeEnabled();
		state.saveStructured.mockResolvedValue({
			sha256: "saved",
			backupPath: "/backup",
		});
		fireEvent.click(screen.getByRole("button", { name: "Save" }));
		await waitFor(() =>
			expect(state.saveStructured).toHaveBeenCalledWith(
				expect.objectContaining({
					expectedSha256: "old",
					settingsPatch: { old: 2 },
				}),
				'{"old":1,"other":true}',
			),
		);
	});
	it("blocks structured mode when raw syntax is invalid", () => {
		render(
			<ConfigEditor
				initial={{
					entry: {
						agent: "codex",
						category: "settings",
						label: "Config",
						path: "/test",
						format: "json",
						exists: true,
					},
					content: "{}",
					sha256: "old",
					secretSpans: [],
				}}
			/>,
		);
		fireEvent.change(
			screen.getByRole("textbox", { name: "Configuration editor" }),
			{ target: { value: "{" } },
		);
		expect(screen.getByRole("button", { name: "Structured" })).toBeDisabled();
		expect(screen.getByRole("alert")).toHaveTextContent("Invalid json");
	});
});
