import {
	act,
	cleanup,
	fireEvent,
	render,
	screen,
	waitFor,
} from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { RemoteConfigEditor } from "./RemoteConfigEditor";

const { api } = vi.hoisted(() => ({
	api: {
		list: vi.fn(),
		read: vi.fn(),
		review: vi.fn(),
		save: vi.fn(),
		verify: vi.fn(),
		message: (e: unknown) => String(e),
	},
}));
vi.mock("@/lib/remote-configs", async (original) => ({
	...(await original<typeof import("@/lib/remote-configs")>()),
	remoteConfigs: api,
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
			aria-label="Remote configuration editor"
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
		<section aria-label="Remote diff">
			{original} → {modified}
		</section>
	),
}));
afterEach(() => {
	cleanup();
	vi.resetAllMocks();
});
const entry = {
	id: "codex-config",
	agent: "codex",
	label: "Codex config",
	format: "toml",
};
const doc = {
	handle: "opened",
	entry,
	host: "host",
	user: "alice",
	port: 22,
	path: "/home/alice/.codex/config.toml",
	content: 'a = "old"',
	sha256: "hash",
	secretSpans: [],
};
const review = {
	reviewId: "reviewed",
	host: "host",
	user: "alice",
	port: 22,
	path: "/home/alice/.codex/config.toml",
	originalContent: 'a = "old"',
	proposedContent: 'a = "new"',
	originalSecretSpans: [],
	proposedSecretSpans: [],
};
async function prepare() {
	api.list.mockResolvedValue([entry]);
	api.read.mockResolvedValue(doc);
	api.review.mockResolvedValue(review);
	render(<RemoteConfigEditor environmentId="env" agent="codex" />);
	fireEvent.click(await screen.findByRole("button", { name: "Codex config" }));
	const editor = await screen.findByRole("textbox", {
		name: "Remote configuration editor",
	});
	return editor;
}
async function reviewed() {
	const editor = await prepare();
	fireEvent.change(editor, { target: { value: 'a = "new"' } });
	fireEvent.click(screen.getByRole("button", { name: "Review remote diff" }));
	await screen.findByRole("button", { name: "Confirm remote save" });
	return editor;
}
describe("remote config editor", () => {
	it("loads only the listed file and reviews a draft before saving", async () => {
		api.list.mockResolvedValue([
			{ id: "codex-config", label: "Codex config", format: "toml" },
		]);
		api.read.mockResolvedValue({
			handle: "opened",
			entry: {
				id: "codex-config",
				agent: "codex",
				label: "Codex config",
				format: "toml",
			},
			host: "host",
			user: "alice",
			port: 22,
			path: "/home/alice/.codex/config.toml",
			content: 'a = "old"',
			sha256: "hash",
			secretSpans: [],
		});
		api.review.mockResolvedValue({
			reviewId: "reviewed",
			host: "host",
			user: "alice",
			port: 22,
			path: "/home/alice/.codex/config.toml",
			originalContent: 'a = "old"',
			proposedContent: 'a = "new"',
			originalSecretSpans: [],
			proposedSecretSpans: [],
		});
		render(<RemoteConfigEditor environmentId="env" agent="codex" />);
		fireEvent.click(
			await screen.findByRole("button", { name: "Codex config" }),
		);
		const editor = await screen.findByRole("textbox", {
			name: "Remote configuration editor",
		});
		fireEvent.change(editor, { target: { value: 'a = "new"' } });
		expect(
			screen.queryByRole("button", { name: "Confirm remote save" }),
		).not.toBeInTheDocument();
		fireEvent.click(screen.getByRole("button", { name: "Review remote diff" }));
		expect(await screen.findByText(/alice@host:22/)).toBeInTheDocument();
		expect(
			screen.getAllByText(/\/home\/alice\/\.codex\/config.toml/)[0],
		).toBeInTheDocument();
		expect(
			screen.getByRole("button", { name: "Confirm remote save" }),
		).toBeEnabled();
	});
	it("reports unreadable files without opening or enabling a save", async () => {
		api.list.mockResolvedValue([entry]);
		api.read.mockRejectedValue("Permission denied");
		render(<RemoteConfigEditor environmentId="env" agent="codex" />);
		fireEvent.click(
			await screen.findByRole("button", { name: "Codex config" }),
		);
		expect(await screen.findByRole("alert")).toHaveTextContent(
			"Permission denied",
		);
		expect(screen.queryByRole("textbox")).not.toBeInTheDocument();
		expect(api.save).not.toHaveBeenCalled();
	});
	it("blocks invalid syntax and never sends a review", async () => {
		const editor = await prepare();
		fireEvent.change(editor, { target: { value: "a = [" } });
		expect(screen.getByRole("alert")).toHaveTextContent("Invalid toml");
		expect(
			screen.getByRole("button", { name: "Review remote diff" }),
		).toBeDisabled();
		expect(api.review).not.toHaveBeenCalled();
	});
	it("masks secret spans in the review without replacing actual content", async () => {
		const editor = await prepare();
		fireEvent.change(editor, { target: { value: 'a = "new"' } });
		api.review.mockResolvedValue({
			...review,
			originalSecretSpans: [{ startByte: 5, endByte: 8, kind: "token" }],
			proposedSecretSpans: [{ startByte: 5, endByte: 8, kind: "token" }],
		});
		fireEvent.click(screen.getByRole("button", { name: "Review remote diff" }));
		expect(
			await screen.findByRole("region", { name: "Remote diff" }),
		).toHaveTextContent('a = "••••••"');
		expect(api.review).toHaveBeenCalledWith("opened", 'a = "new"');
	});
	it("invalidates a confirmation when the draft changes", async () => {
		const editor = await reviewed();
		fireEvent.change(editor, { target: { value: 'a = "later"' } });
		expect(
			screen.queryByRole("button", { name: "Confirm remote save" }),
		).not.toBeInTheDocument();
		expect(api.save).not.toHaveBeenCalled();
	});
	it("preserves a draft across conflict and reload", async () => {
		const editor = await reviewed();
		api.save.mockRejectedValue("ExternalConflict");
		api.verify.mockResolvedValue({
			...doc,
			handle: "new-handle",
			content: 'a = "disk"',
			sha256: "new-hash",
		});
		fireEvent.click(
			screen.getByRole("button", { name: "Confirm remote save" }),
		);
		fireEvent.click(
			await screen.findByRole("button", { name: "Reload from remote" }),
		);
		expect(await screen.findByText(/draft is preserved/)).toBeInTheDocument();
		expect(editor).toHaveValue('a = "new"');
		expect(api.save).toHaveBeenCalledTimes(1);
	});
	it("verifies an unconfirmed result without automatically retrying", async () => {
		const editor = await reviewed();
		api.save.mockRejectedValue("RemoteUnconfirmed");
		api.verify.mockResolvedValue({
			...doc,
			handle: "new-handle",
			content: 'a = "new"',
			sha256: "new-hash",
		});
		fireEvent.click(
			screen.getByRole("button", { name: "Confirm remote save" }),
		);
		expect(
			await screen.findByText(/save result is unconfirmed/i),
		).toBeInTheDocument();
		expect(editor).toHaveValue('a = "new"');
		fireEvent.click(
			screen.getByRole("button", { name: "Verify remote content" }),
		);
		expect(
			await screen.findByText(/current remote content matches/),
		).toBeInTheDocument();
		expect(api.save).toHaveBeenCalledTimes(1);
	});
	it("requires reopening when identity changes and reports write errors", async () => {
		await reviewed();
		api.save.mockRejectedValue("RemoteDestinationChanged");
		fireEvent.click(
			screen.getByRole("button", { name: "Confirm remote save" }),
		);
		expect(
			await screen.findByText(/remote destination changed/i),
		).toBeInTheDocument();
		expect(
			screen.getByRole("button", { name: "Review remote diff" }),
		).toBeDisabled();
		expect(api.save).toHaveBeenCalledTimes(1);
	});
	it("discards an old read after the environment changes", async () => {
		api.list.mockResolvedValue([entry]);
		let resolve: (value: unknown) => void = () => {};
		api.read.mockReturnValue(
			new Promise((done) => {
				resolve = done;
			}),
		);
		const view = render(
			<RemoteConfigEditor environmentId="old" agent="codex" />,
		);
		fireEvent.click(
			await screen.findByRole("button", { name: "Codex config" }),
		);
		view.rerender(<RemoteConfigEditor environmentId="new" agent="codex" />);
		await act(async () => resolve(doc));
		await waitFor(() =>
			expect(screen.queryByRole("textbox")).not.toBeInTheDocument(),
		);
	});
	it("rejects editing protected project data", async () => {
		const item = {
			id: "claude-home",
			agent: "claude-code",
			label: ".claude.json",
			format: "json",
		};
		api.list.mockResolvedValue([item]);
		api.read.mockResolvedValue({
			...doc,
			entry: item,
			content: '{"projects":{"x":1},"other":1}',
		});
		render(<RemoteConfigEditor environmentId="env" agent="claude-code" />);
		fireEvent.click(
			await screen.findByRole("button", { name: ".claude.json" }),
		);
		const editor = await screen.findByRole("textbox");
		fireEvent.change(editor, {
			target: { value: '{"projects":{"x":2},"other":1}' },
		});
		expect(screen.getByRole("alert")).toHaveTextContent(/project settings/i);
		expect(
			screen.getByRole("button", { name: "Review remote diff" }),
		).toBeDisabled();
	});
	it("reports a write failure and requires a fresh remote read before another review", async () => {
		const editor = await reviewed();
		api.save.mockRejectedValue({ IoError: { message: "backup failed" } });
		fireEvent.click(
			screen.getByRole("button", { name: "Confirm remote save" }),
		);
		expect(await screen.findByText(/Save failed/)).toBeInTheDocument();
		expect(
			screen.getByRole("button", { name: "Review remote diff" }),
		).toBeDisabled();
		expect(editor).toHaveValue('a = "new"');
		expect(api.save).toHaveBeenCalledTimes(1);
	});
	it("sends only the approved review identifier and confirmation on save", async () => {
		await reviewed();
		api.save.mockResolvedValue({ sha256: "next", backupPath: "/backup" });
		fireEvent.click(
			screen.getByRole("button", { name: "Confirm remote save" }),
		);
		expect(
			await screen.findByText(/Remote save confirmed/),
		).toBeInTheDocument();
		expect(api.save).toHaveBeenCalledWith("reviewed");
	});
	it("rejects a review for a different authenticated target", async () => {
		const editor = await prepare();
		api.review.mockResolvedValue({ ...review, host: "different" });
		fireEvent.change(editor, { target: { value: 'a = "new"' } });
		fireEvent.click(screen.getByRole("button", { name: "Review remote diff" }));
		expect(
			await screen.findByText(/remote destination changed/i),
		).toBeInTheDocument();
		expect(
			screen.queryByRole("button", { name: "Confirm remote save" }),
		).not.toBeInTheDocument();
	});
});
