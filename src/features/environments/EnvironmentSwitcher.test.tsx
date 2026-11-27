import {
	act,
	cleanup,
	fireEvent,
	render,
	screen,
} from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { EnvironmentSwitcher } from "./EnvironmentSwitcher";

const { api } = vi.hoisted(() => ({
	api: {
		list: vi.fn(),
		inspect: vi.fn(),
		test: vi.fn(),
		save: vi.fn(),
		remove: vi.fn(),
		message: (e: unknown) => String(e),
	},
}));
vi.mock("@/lib/environments", async (original) => ({
	...(await original<typeof import("@/lib/environments")>()),
	environments: api,
}));
afterEach(() => {
	cleanup();
	vi.resetAllMocks();
});
describe("environment switcher", () => {
	it("discards an inventory response after returning to Local", async () => {
		api.list.mockResolvedValue([
			{
				id: "remote",
				name: "Remote",
				connection: { mode: "alias", alias: "host" },
			},
		]);
		let resolve: (value: unknown) => void = () => {};
		api.inspect.mockReturnValue(
			new Promise((done) => {
				resolve = done;
			}),
		);
		render(
			<EnvironmentSwitcher>
				<p>Local content</p>
			</EnvironmentSwitcher>,
		);
		await screen.findByRole("option", { name: "Remote" });
		fireEvent.change(screen.getByLabelText("Environment"), {
			target: { value: "remote" },
		});
		expect(await screen.findByText(/Checking remote/)).toBeInTheDocument();
		fireEvent.change(screen.getByLabelText("Environment"), {
			target: { value: "local" },
		});
		await act(async () =>
			resolve({
				host: "old-host",
				user: "old-user",
				port: 22,
				claude: true,
				codex: true,
			}),
		);
		expect(screen.getByText("Local content")).toBeInTheDocument();
		expect(screen.queryByText(/old-host/)).not.toBeInTheDocument();
	});
	it("rejects option-like aliases before testing a connection", async () => {
		api.list.mockResolvedValue([]);
		render(
			<EnvironmentSwitcher>
				<p>Local content</p>
			</EnvironmentSwitcher>,
		);
		fireEvent.click(
			screen.getByRole("button", { name: "Add SSH environment" }),
		);
		fireEvent.change(screen.getByLabelText("Display name"), {
			target: { value: "Server" },
		});
		fireEvent.change(screen.getByLabelText("SSH alias"), {
			target: { value: "-bad" },
		});
		fireEvent.click(screen.getByRole("button", { name: "Test connection" }));
		expect(screen.getByRole("alert")).toHaveTextContent(/valid alias/i);
		expect(api.test).not.toHaveBeenCalled();
	});
	it("tests alias and manual connections without saving", async () => {
		api.list.mockResolvedValue([]);
		api.test.mockResolvedValue({
			host: "server.example",
			user: "alice",
			port: 2222,
			claude: true,
			codex: false,
		});
		render(
			<EnvironmentSwitcher>
				<p>Local</p>
			</EnvironmentSwitcher>,
		);
		fireEvent.click(
			screen.getByRole("button", { name: "Add SSH environment" }),
		);
		fireEvent.change(screen.getByLabelText("Display name"), {
			target: { value: "Work" },
		});
		fireEvent.change(screen.getByLabelText("SSH alias"), {
			target: { value: "work" },
		});
		fireEvent.click(screen.getByRole("button", { name: "Test connection" }));
		expect(
			await screen.findByText(/alice@server.example:2222/),
		).toBeInTheDocument();
		expect(api.test).toHaveBeenCalledWith({
			name: "Work",
			connection: { mode: "alias", alias: "work" },
		});
		expect(api.save).not.toHaveBeenCalled();
		fireEvent.change(screen.getByLabelText("Connection mode"), {
			target: { value: "manual" },
		});
		fireEvent.change(screen.getByLabelText("Host"), {
			target: { value: "server.example" },
		});
		fireEvent.change(screen.getByLabelText("User"), {
			target: { value: "alice" },
		});
		fireEvent.change(screen.getByLabelText("Port"), {
			target: { value: "2222" },
		});
		fireEvent.click(screen.getByRole("button", { name: "Test connection" }));
		expect(
			await screen.findByText(/alice@server.example:2222/),
		).toBeInTheDocument();
		expect(api.test).toHaveBeenLastCalledWith({
			name: "Work",
			connection: {
				mode: "manual",
				host: "server.example",
				user: "alice",
				port: 2222,
			},
		});
		expect(api.save).not.toHaveBeenCalled();
	});
	it("shows connection failure without claiming agents are absent", async () => {
		api.list.mockResolvedValue([
			{ id: "r", name: "Remote", connection: { mode: "alias", alias: "host" } },
		]);
		api.inspect.mockRejectedValue("Host key verification failed");
		render(
			<EnvironmentSwitcher>
				<p>Local</p>
			</EnvironmentSwitcher>,
		);
		await screen.findByRole("option", { name: "Remote" });
		fireEvent.change(screen.getByLabelText("Environment"), {
			target: { value: "r" },
		});
		expect(await screen.findByRole("alert")).toHaveTextContent(
			"Host key verification failed",
		);
		expect(screen.queryByText("Not detected")).not.toBeInTheDocument();
	});
	it("discards stale test results after changing the connection", async () => {
		api.list.mockResolvedValue([]);
		let resolve: (value: unknown) => void = () => {};
		api.test.mockReturnValue(
			new Promise((done) => {
				resolve = done;
			}),
		);
		render(
			<EnvironmentSwitcher>
				<p>Local</p>
			</EnvironmentSwitcher>,
		);
		fireEvent.click(
			screen.getByRole("button", { name: "Add SSH environment" }),
		);
		fireEvent.change(screen.getByLabelText("Display name"), {
			target: { value: "Work" },
		});
		fireEvent.change(screen.getByLabelText("SSH alias"), {
			target: { value: "first" },
		});
		fireEvent.click(screen.getByRole("button", { name: "Test connection" }));
		fireEvent.change(screen.getByLabelText("SSH alias"), {
			target: { value: "second" },
		});
		await act(async () =>
			resolve({
				host: "stale-host",
				user: "a",
				port: 22,
				claude: true,
				codex: true,
			}),
		);
		expect(screen.queryByText(/stale-host/)).not.toBeInTheDocument();
	});
	it("rejects an invalid manual port before testing", () => {
		api.list.mockResolvedValue([]);
		render(
			<EnvironmentSwitcher>
				<p>Local</p>
			</EnvironmentSwitcher>,
		);
		fireEvent.click(
			screen.getByRole("button", { name: "Add SSH environment" }),
		);
		fireEvent.change(screen.getByLabelText("Display name"), {
			target: { value: "Work" },
		});
		fireEvent.change(screen.getByLabelText("Connection mode"), {
			target: { value: "manual" },
		});
		fireEvent.change(screen.getByLabelText("Host"), {
			target: { value: "host" },
		});
		fireEvent.change(screen.getByLabelText("User"), {
			target: { value: "alice" },
		});
		fireEvent.change(screen.getByLabelText("Port"), {
			target: { value: "65536" },
		});
		fireEvent.click(screen.getByRole("button", { name: "Test connection" }));
		expect(screen.getByRole("alert")).toHaveTextContent(/port between/);
		expect(api.test).not.toHaveBeenCalled();
	});
	it("saves, selects, and removes an SSH registration without affecting the server", async () => {
		api.list.mockResolvedValue([]);
		const record = {
			id: "ssh1",
			name: "Server",
			connection: { mode: "alias", alias: "server" },
		};
		api.save.mockResolvedValue(record);
		api.remove.mockResolvedValue(undefined);
		api.inspect.mockResolvedValue({
			host: "remote",
			user: "bob",
			port: 22,
			claude: true,
			codex: false,
		});
		render(
			<EnvironmentSwitcher>
				<p>Local content</p>
			</EnvironmentSwitcher>,
		);
		fireEvent.click(
			screen.getByRole("button", { name: "Add SSH environment" }),
		);
		fireEvent.change(screen.getByLabelText("Display name"), {
			target: { value: "Server" },
		});
		fireEvent.change(screen.getByLabelText("SSH alias"), {
			target: { value: "server" },
		});
		fireEvent.click(screen.getByRole("button", { name: "Save environment" }));
		expect(await screen.findByText("bob@remote:22")).toBeInTheDocument();
		expect(screen.getByText("Detected")).toBeInTheDocument();
		expect(screen.getByText("Not detected")).toBeInTheDocument();
		fireEvent.click(screen.getByRole("button", { name: "Remove environment" }));
		expect(screen.getByText(/server will not be changed/i)).toBeInTheDocument();
		fireEvent.click(screen.getByRole("button", { name: "Confirm removal" }));
		expect(await screen.findByText("Local content")).toBeInTheDocument();
		expect(api.remove).toHaveBeenCalledWith("ssh1");
	});
	it("discards remote inventory arriving after the active registration is edited", async () => {
		api.list.mockResolvedValue([
			{ id: "r", name: "Remote", connection: { mode: "alias", alias: "host" } },
		]);
		let resolve: (value: unknown) => void = () => {};
		api.inspect.mockReturnValue(
			new Promise((done) => {
				resolve = done;
			}),
		);
		render(
			<EnvironmentSwitcher>
				<p>Local</p>
			</EnvironmentSwitcher>,
		);
		await screen.findByRole("option", { name: "Remote" });
		fireEvent.change(screen.getByLabelText("Environment"), {
			target: { value: "r" },
		});
		fireEvent.click(screen.getByRole("button", { name: "Edit environment" }));
		fireEvent.change(screen.getByLabelText("SSH alias"), {
			target: { value: "changed" },
		});
		await act(async () =>
			resolve({
				host: "stale",
				user: "user",
				port: 22,
				claude: true,
				codex: true,
			}),
		);
		expect(screen.queryByText(/user@stale/)).not.toBeInTheDocument();
	});
	it("returns to Local and ignores a pending inventory after removal", async () => {
		api.list.mockResolvedValue([
			{ id: "r", name: "Remote", connection: { mode: "alias", alias: "host" } },
		]);
		api.remove.mockResolvedValue(undefined);
		let resolve: (value: unknown) => void = () => {};
		api.inspect.mockReturnValue(
			new Promise((done) => {
				resolve = done;
			}),
		);
		render(
			<EnvironmentSwitcher>
				<p>Local content</p>
			</EnvironmentSwitcher>,
		);
		await screen.findByRole("option", { name: "Remote" });
		fireEvent.change(screen.getByLabelText("Environment"), {
			target: { value: "r" },
		});
		fireEvent.click(screen.getByRole("button", { name: "Remove environment" }));
		fireEvent.click(screen.getByRole("button", { name: "Confirm removal" }));
		expect(await screen.findByText("Local content")).toBeInTheDocument();
		await act(async () =>
			resolve({
				host: "stale",
				user: "user",
				port: 22,
				claude: true,
				codex: true,
			}),
		);
		expect(screen.queryByText(/user@stale/)).not.toBeInTheDocument();
	});
});
