import {
	cleanup,
	fireEvent,
	render,
	screen,
	waitFor,
	within,
} from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { DesktopQuotaState_Serialize } from "@/bindings/tauri";
import { ClaudeAccountQuotas } from "./ClaudeAccountQuotas";

const api = vi.hoisted(() => ({
	list: vi.fn(),
	create: vi.fn(),
	open: vi.fn(),
	link: vi.fn(),
	refresh: vi.fn(),
}));
vi.mock("@/bindings/tauri", () => ({
	commands: {
		listClaudeDesktopQuotaProfiles: api.list,
		createClaudeDesktopQuotaProfile: api.create,
		openClaudeDesktopQuotaLogin: api.open,
		linkClaudeDesktopQuotaAccount: api.link,
		refreshClaudeDesktopQuota: api.refresh,
	},
}));
const empty: DesktopQuotaState_Serialize = {
	account: null,
	last_attempt_at: null,
	last_success: null,
	error: null,
};
const historical: DesktopQuotaState_Serialize = {
	account: { display_name: "Personal" },
	last_attempt_at: "2026-11-27T10:00:00Z",
	last_success: {
		collected_at: "2026-11-27T09:00:00Z",
		five_hour: { utilization: 0, resets_at: null },
		seven_day: { utilization: null, resets_at: "2026-11-30T19:00:00Z" },
	},
	error: null,
};
const profile = (id: string, state = empty, is_default = false) => ({
	id,
	state,
	is_default,
});
const result = (
	status: string,
	state = historical,
	retry_after_seconds: number | null = null,
) => ({ status, state, retry_after_seconds });
beforeEach(() => {
	vi.clearAllMocks();
	api.list.mockResolvedValue([profile("default", empty, true)]);
	api.create.mockResolvedValue(profile("new"));
	api.open.mockResolvedValue(undefined);
	api.link.mockResolvedValue(profile("default", historical, true));
	api.refresh.mockResolvedValue(result("updated"));
});
afterEach(cleanup);

describe("Claude Desktop quota", () => {
	it("binds the default account without collecting", async () => {
		render(<ClaudeAccountQuotas />);
		fireEvent.click(
			await screen.findByRole("button", { name: /link desktop account/i }),
		);
		expect(await screen.findByText("Personal")).toBeInTheDocument();
		expect(api.link).toHaveBeenCalledWith("default");
		expect(api.refresh).not.toHaveBeenCalled();
	});
	it("creates an isolated draft, opens login and confirms only when requested", async () => {
		render(<ClaudeAccountQuotas />);
		fireEvent.click(
			await screen.findByRole("button", { name: /add account/i }),
		);
		await waitFor(() => expect(api.open).toHaveBeenCalledWith("new"));
		expect(api.link).not.toHaveBeenCalled();
		api.link.mockResolvedValue(profile("new", historical));
		fireEvent.click(
			await screen.findByRole("button", { name: /confirm account/i }),
		);
		expect(await screen.findByText("Personal")).toBeInTheDocument();
		expect(api.link).toHaveBeenCalledWith("new");
		expect(api.refresh).not.toHaveBeenCalled();
	});
	it("keeps a draft when login fails and allows resuming", async () => {
		api.open.mockRejectedValueOnce("source_unavailable");
		render(<ClaudeAccountQuotas />);
		fireEvent.click(
			await screen.findByRole("button", { name: /add account/i }),
		);
		expect(await screen.findByRole("alert")).toBeInTheDocument();
		fireEvent.click(screen.getByRole("button", { name: /open login/i }));
		await waitFor(() => expect(api.open).toHaveBeenCalledTimes(2));
		expect(api.create).toHaveBeenCalledTimes(1);
	});
	it("restores drafts on entry and displays duplicate identity errors", async () => {
		api.list.mockResolvedValue([
			profile("default", empty, true),
			profile("draft"),
		]);
		api.link.mockRejectedValue("duplicate_account");
		render(<ClaudeAccountQuotas />);
		fireEvent.click(
			await screen.findByRole("button", { name: /confirm account/i }),
		);
		expect(await screen.findByRole("alert")).toHaveTextContent(/already/i);
		expect(api.open).not.toHaveBeenCalled();
	});
	it("refreshes accounts independently and preserves historical partial values after failure", async () => {
		const work = { ...historical, account: { display_name: "Work" } };
		api.list.mockResolvedValue([
			profile("default", historical, true),
			profile("work", work),
		]);
		api.refresh.mockImplementation(async (id: string) =>
			id === "default"
				? result("failed", { ...historical, error: "identity_mismatch" })
				: result("updated", work),
		);
		render(<ClaudeAccountQuotas />);
		expect(await screen.findByText(/identity/i)).toBeInTheDocument();
		expect(await screen.findByText("Work")).toBeInTheDocument();
		expect(screen.getByText(/historical/i)).toBeInTheDocument();
		const groups = screen.getAllByRole("group", { name: /5 hour/i });
		expect(within(groups[0]).getByText("0%")).toBeInTheDocument();
		expect(within(groups[0]).getByText(/unavailable/i)).toBeInTheDocument();
		expect(api.refresh).toHaveBeenCalledWith("default");
		expect(api.refresh).toHaveBeenCalledWith("work");
	});
	it("refreshes all only on entry and explicit action without polling", async () => {
		api.list.mockResolvedValue([
			profile("default", historical, true),
			profile("work", { ...historical, account: { display_name: "Work" } }),
		]);
		api.refresh.mockImplementation(async (id: string) =>
			result(
				"wait",
				id === "work"
					? { ...historical, account: { display_name: "Work" } }
					: historical,
				30,
			),
		);
		render(<ClaudeAccountQuotas />);
		await waitFor(() => expect(api.refresh).toHaveBeenCalledTimes(2));
		fireEvent.focus(window);
		await new Promise((resolve) => setTimeout(resolve, 20));
		expect(api.refresh).toHaveBeenCalledTimes(2);
		expect(screen.getAllByText(/wait.*30/i)).toHaveLength(2);
		fireEvent.click(screen.getByRole("button", { name: /refresh all/i }));
		await waitFor(() => expect(api.refresh).toHaveBeenCalledTimes(4));
	});
	it("offers isolated signing in again after expiry without opening login automatically", async () => {
		api.list.mockResolvedValue([
			profile("expired", { ...historical, error: "unauthenticated" }),
		]);
		api.refresh.mockResolvedValue(
			result("failed", { ...historical, error: "unauthenticated" }),
		);
		render(<ClaudeAccountQuotas />);
		expect(await screen.findByRole("alert")).toHaveTextContent(/sign in/i);
		expect(api.open).not.toHaveBeenCalled();
		fireEvent.click(screen.getByRole("button", { name: /open login/i }));
		await waitFor(() => expect(api.open).toHaveBeenCalledWith("expired"));
	});
	it("does not queue a second refresh for an in-flight account", async () => {
		let finish!: (value: ReturnType<typeof result>) => void;
		api.list.mockResolvedValue([profile("default", historical, true)]);
		api.refresh.mockReturnValue(
			new Promise((resolve) => {
				finish = resolve;
			}),
		);
		render(<ClaudeAccountQuotas />);
		await waitFor(() => expect(api.refresh).toHaveBeenCalledTimes(1));
		fireEvent.click(screen.getByRole("button", { name: /refresh all/i }));
		expect(api.refresh).toHaveBeenCalledTimes(1);
		finish(result("updated"));
		await screen.findByText(/latest collection/i);
	});
});
