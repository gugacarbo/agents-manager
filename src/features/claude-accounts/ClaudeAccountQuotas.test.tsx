import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import {
	cleanup,
	fireEvent,
	render,
	screen,
	waitFor,
	within,
} from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type {
	DesktopQuotaProfile_Serialize,
	DesktopQuotaRefresh_Serialize,
	DesktopQuotaState_Serialize,
} from "@/lib/claude-accounts";
import { ClaudeAccountQuotas } from "./ClaudeAccountQuotas";

const api = vi.hoisted(() => ({
	list: vi.fn(),
	create: vi.fn(),
	open: vi.fn(),
	openProfile: vi.fn(),
	link: vi.fn(),
	refresh: vi.fn(),
}));
vi.mock("@/lib/claude-accounts", () => ({
	listClaudeDesktopQuotaProfiles: api.list,
	createClaudeDesktopQuotaProfile: api.create,
	openClaudeDesktopQuotaLogin: api.open,
	openClaudeDesktopQuotaProfile: api.openProfile,
	linkClaudeDesktopQuotaAccount: api.link,
	refreshClaudeDesktopQuota: api.refresh,
}));

function renderQuotas() {
	const client = new QueryClient({
		defaultOptions: {
			queries: {
				retry: false,
			},
		},
	});
	return render(
		<QueryClientProvider client={client}>
			<ClaudeAccountQuotas />
		</QueryClientProvider>,
	);
}

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
const profile = (
	id: string,
	state = empty,
	is_default = false,
	shares_default_quota = false,
): DesktopQuotaProfile_Serialize => ({
	id,
	state,
	is_default,
	shares_default_quota,
});
const result = (
	status: DesktopQuotaRefresh_Serialize["status"],
	state = historical,
	retry_after_seconds: number | null = null,
): DesktopQuotaRefresh_Serialize => ({ status, state, retry_after_seconds });
beforeEach(() => {
	vi.clearAllMocks();
	api.list.mockResolvedValue([profile("default", empty, true)]);
	api.create.mockResolvedValue(profile("new"));
	api.open.mockResolvedValue(undefined);
	api.openProfile.mockResolvedValue(undefined);
	api.link.mockResolvedValue(profile("default", historical, true));
	api.refresh.mockResolvedValue(result("updated"));
});
afterEach(cleanup);

describe("Claude Desktop quota", () => {
	it("adds an isolated opening to the default account without creating another quota row", async () => {
		api.list.mockResolvedValue([
			profile("default", historical, true),
			profile("draft"),
		]);
		api.link.mockResolvedValue(profile("draft", historical, false, true));
		renderQuotas();
		fireEvent.click(
			await screen.findByRole("button", { name: "Confirm account" }),
		);
		const isolated = await screen.findByRole("button", {
			name: "Open isolated profile",
		});
		expect(
			screen.getAllByRole("button", { name: "Refresh quota" }),
		).toHaveLength(1);
		expect(
			screen.getByRole("button", { name: "Open default session" }),
		).toBeVisible();
		await waitFor(() => expect(isolated).toBeEnabled());
		fireEvent.click(isolated);
		await waitFor(() => expect(api.openProfile).toHaveBeenCalledWith("draft"));
		expect(api.refresh).toHaveBeenCalledTimes(1);
		expect(api.refresh).toHaveBeenCalledWith("default");
	});
	it("binds the default account without collecting", async () => {
		renderQuotas();
		fireEvent.click(
			await screen.findByRole("button", { name: /link desktop account/i }),
		);
		expect(await screen.findByText("Personal")).toBeInTheDocument();
		expect(api.link).toHaveBeenCalledWith("default");
		expect(api.refresh).not.toHaveBeenCalled();
	});
	it("identifies the keyring when linking fails to unlock the session", async () => {
		api.link.mockRejectedValueOnce("keyring_unavailable");
		renderQuotas();
		fireEvent.click(
			await screen.findByRole("button", { name: /link desktop account/i }),
		);
		expect(await screen.findByRole("alert")).toHaveTextContent(/secret-tool/i);
	});
	it("opens the selected linked profile and prevents a second switch while it is starting", async () => {
		const work = { ...historical, account: { display_name: "Work" } };
		api.list.mockResolvedValue([
			profile("default", historical, true),
			profile("work", work),
		]);
		api.refresh.mockImplementation(async (id: string) =>
			result("updated", id === "work" ? work : historical),
		);
		let finish!: () => void;
		api.openProfile.mockReturnValue(
			new Promise<void>((resolve) => {
				finish = resolve;
			}),
		);
		renderQuotas();
		await waitFor(() =>
			expect(screen.getAllByText(/Latest collection/)).toHaveLength(2),
		);
		fireEvent.click(await screen.findByRole("button", { name: "Open Work" }));
		fireEvent.click(screen.getByRole("button", { name: "Open Personal" }));
		await waitFor(() => expect(api.openProfile).toHaveBeenCalledTimes(1));
		expect(api.openProfile).toHaveBeenCalledWith("work");
		expect(screen.getAllByText(/Latest collection/)).toHaveLength(2);
		finish();
		await waitFor(() =>
			expect(
				screen.getByRole("button", { name: "Open Personal" }),
			).toBeEnabled(),
		);
		fireEvent.click(screen.getByRole("button", { name: "Open Personal" }));
		await waitFor(() => expect(api.openProfile).toHaveBeenCalledTimes(2));
		expect(api.openProfile).toHaveBeenLastCalledWith("default");
	});
	it("disables opening a profile while its quota refresh is in progress", async () => {
		let finish!: (value: ReturnType<typeof result>) => void;
		api.list.mockResolvedValue([profile("work", historical)]);
		api.refresh.mockReturnValue(
			new Promise((resolve) => {
				finish = resolve;
			}),
		);
		renderQuotas();
		expect(
			await screen.findByRole("button", { name: "Open Personal" }),
		).toBeDisabled();
		expect(api.openProfile).not.toHaveBeenCalled();
		finish(result("updated"));
	});
	it("explains that Claude Desktop must be closed when opening a profile", async () => {
		const work = { ...historical, account: { display_name: "Work" } };
		api.openProfile.mockRejectedValueOnce("desktop_running");
		api.list.mockResolvedValue([profile("work", work)]);
		api.refresh.mockResolvedValue(result("updated", work));
		renderQuotas();
		fireEvent.click(await screen.findByRole("button", { name: "Open Work" }));
		expect(await screen.findByRole("alert")).toHaveTextContent(
			/close Claude Desktop/i,
		);
	});
	it("does not offer opening for an unlinked pending profile", async () => {
		api.list.mockResolvedValue([profile("draft")]);
		renderQuotas();
		await screen.findByText("Sign-in pending");
		const pendingRow = screen.getByRole("region", {
			name: "Pending Desktop account",
		});
		expect(
			within(pendingRow)
				.getAllByRole("button")
				.map((button) => button.textContent),
		).toEqual(["Open login", "Confirm account"]);
	});
	it("explains how to recover when a linked profile is no longer available", async () => {
		api.openProfile.mockRejectedValueOnce("profile_not_found");
		api.list.mockResolvedValue([profile("work", historical)]);
		renderQuotas();
		fireEvent.click(
			await screen.findByRole("button", { name: "Open Personal" }),
		);
		expect(await screen.findByRole("alert")).toHaveTextContent(
			/reopen the accounts screen/i,
		);
	});
	it("offers a new sign-in when the linked profile session expires", async () => {
		api.openProfile.mockRejectedValueOnce("unauthenticated");
		api.list.mockResolvedValue([profile("work", historical)]);
		renderQuotas();
		await screen.findByText(/Latest collection/);
		fireEvent.click(
			await screen.findByRole("button", { name: "Open Personal" }),
		);
		expect(await screen.findByRole("alert")).toHaveTextContent(
			/use open login/i,
		);
		expect(screen.getByRole("button", { name: "Open login" })).toBeVisible();
	});
	it("directs the user to sign in to the default session through Claude Desktop", async () => {
		api.openProfile.mockRejectedValueOnce("unauthenticated");
		api.list.mockResolvedValue([profile("default", historical, true)]);
		renderQuotas();
		await screen.findByText(/Latest collection/);
		fireEvent.click(
			await screen.findByRole("button", { name: "Open Personal" }),
		);
		expect(await screen.findByRole("alert")).toHaveTextContent(
			/sign in through the default Claude Desktop window/i,
		);
	});
	it("allows retrying a linked profile after a launch failure", async () => {
		api.openProfile.mockRejectedValueOnce("source_unavailable");
		api.list.mockResolvedValue([profile("work", historical)]);
		renderQuotas();
		await screen.findByText(/Latest collection/);
		fireEvent.click(
			await screen.findByRole("button", { name: "Open Personal" }),
		);
		expect(await screen.findByRole("alert")).toHaveTextContent(
			/could not start Claude Desktop/i,
		);
		fireEvent.click(screen.getByRole("button", { name: "Open Personal" }));
		await waitFor(() => expect(api.openProfile).toHaveBeenCalledTimes(2));
		expect(screen.queryByRole("alert")).not.toBeInTheDocument();
	});
	it("creates an isolated draft, opens login and confirms only when requested", async () => {
		renderQuotas();
		const addButton = screen.getByRole("button", { name: /add account/i });
		await waitFor(() => expect(addButton).toBeEnabled());
		fireEvent.click(addButton);
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
		renderQuotas();
		const addButton = screen.getByRole("button", { name: /add account/i });
		await waitFor(() => expect(addButton).toBeEnabled());
		fireEvent.click(addButton);
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
		renderQuotas();
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
		renderQuotas();
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
		renderQuotas();
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
		renderQuotas();
		expect(await screen.findByRole("alert")).toHaveTextContent(/sign in/i);
		expect(api.open).not.toHaveBeenCalled();
		expect(screen.getByRole("button", { name: "Open Personal" })).toBeVisible();
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
		renderQuotas();
		await waitFor(() => expect(api.refresh).toHaveBeenCalledTimes(1));
		fireEvent.click(screen.getByRole("button", { name: /refresh all/i }));
		expect(api.refresh).toHaveBeenCalledTimes(1);
		finish(result("updated"));
		await screen.findByText(/latest collection/i);
	});
});
