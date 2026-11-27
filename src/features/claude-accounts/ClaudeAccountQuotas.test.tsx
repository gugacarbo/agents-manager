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
	listConversations: vi.fn(),
	importConversations: vi.fn(),
}));
vi.mock("@/lib/claude-accounts", () => ({
	listClaudeDesktopQuotaProfiles: api.list,
	createClaudeDesktopQuotaProfile: api.create,
	openClaudeDesktopQuotaLogin: api.open,
	openClaudeDesktopQuotaProfile: api.openProfile,
	linkClaudeDesktopQuotaAccount: api.link,
	refreshClaudeDesktopQuota: api.refresh,
	listClaudeDesktopConversations: api.listConversations,
	importClaudeDesktopConversations: api.importConversations,
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
	api.listConversations.mockResolvedValue({
		sessions: [],
		unavailableOrigins: [],
	});
	api.importConversations.mockResolvedValue([]);
});
afterEach(cleanup);

describe("Claude Desktop quota", () => {
	const work = { ...historical, account: { display_name: "Work" } };
	const twoAccounts = () => {
		api.list.mockResolvedValue([
			profile("default", historical, true),
			profile("work", work),
		]);
		api.refresh.mockImplementation(async (id: string) =>
			result("updated", id === "work" ? work : historical),
		);
	};
	const enableImport = async () => {
		fireEvent.click(
			await screen.findByRole("checkbox", {
				name: "Import conversations when switching",
			}),
		);
	};

	it("lists nothing until the import opt-in is enabled, and cancelling never lists or imports", async () => {
		twoAccounts();
		renderQuotas();
		await screen.findAllByText(/latest collection/i);
		expect(api.listConversations).not.toHaveBeenCalled();
		expect(
			screen.queryByRole("combobox", { name: "Source account" }),
		).toBeNull();
		fireEvent.click(
			screen.getByRole("checkbox", {
				name: "Import conversations when switching",
			}),
		);
		fireEvent.click(screen.getByRole("button", { name: "Cancel" }));
		expect(api.listConversations).not.toHaveBeenCalled();
		expect(api.importConversations).not.toHaveBeenCalled();
		expect(
			screen.queryByRole("combobox", { name: "Source account" }),
		).toBeNull();
		// The simple switch stays available.
		const openWork = await screen.findByRole("button", { name: "Open Work" });
		expect(openWork).toBeVisible();
		fireEvent.click(openWork);
		await waitFor(() => expect(api.openProfile).toHaveBeenCalledWith("work"));
	});

	it("selects and confirms conversation imports before switching, then shows per-session results", async () => {
		twoAccounts();
		api.listConversations.mockResolvedValue({
			sessions: [
				{
					id: "session-a",
					title: "Design review",
					origin: "SSH: build host",
					available: true,
					reason: null,
				},
				{
					id: "session-b",
					title: "Active session",
					origin: "Local",
					available: false,
					reason: "Session is active",
				},
			],
			unavailableOrigins: [],
		});
		api.importConversations.mockResolvedValue([
			{ id: "session-a", status: "imported", reason: null },
		]);
		renderQuotas();
		await screen.findAllByText(/latest collection/i);
		await enableImport();
		fireEvent.change(screen.getByRole("combobox", { name: "Source account" }), {
			target: { value: "default" },
		});
		fireEvent.change(
			screen.getByRole("combobox", { name: "Destination account" }),
			{
				target: { value: "work" },
			},
		);
		fireEvent.click(screen.getByRole("button", { name: "Find conversations" }));
		expect(await screen.findByText("Design review")).toBeVisible();
		expect(screen.getByText("SSH: build host")).toBeVisible();
		expect(screen.getByText(/Session is active/)).toBeVisible();
		const checkbox = screen.getByRole("checkbox", { name: /Design review/ });
		fireEvent.click(checkbox);
		fireEvent.click(
			screen.getByRole("button", { name: "Import selected conversations" }),
		);
		expect(
			await screen.findByRole("dialog", {
				name: "Confirm conversation import",
			}),
		).toBeVisible();
		fireEvent.click(screen.getByRole("button", { name: "Confirm import" }));
		expect(await screen.findByText(/Imported: Design review/)).toBeVisible();
		expect(api.importConversations).toHaveBeenCalledWith("default", "work", [
			"session-a",
		]);
		expect(
			within(
				screen.getByRole("region", { name: "Conversation import" }),
			).getByRole("button", { name: "Open destination account" }),
		).toBeVisible();
	});

	it("retries only unavailable conversations after a partial import", async () => {
		twoAccounts();
		api.listConversations.mockResolvedValue({
			sessions: [
				{
					id: "ready",
					title: "Ready session",
					origin: "Local",
					available: true,
					reason: null,
				},
				{
					id: "remote",
					title: "Remote session",
					origin: "SSH: builder",
					available: true,
					reason: null,
				},
			],
			unavailableOrigins: [],
		});
		api.importConversations
			.mockResolvedValueOnce([
				{ id: "ready", status: "imported", reason: null },
				{ id: "remote", status: "unavailable", reason: "Host is offline" },
			])
			.mockResolvedValueOnce([
				{ id: "remote", status: "imported", reason: null },
			]);
		renderQuotas();
		await screen.findAllByText(/latest collection/i);
		await enableImport();
		fireEvent.click(screen.getByRole("button", { name: "Find conversations" }));
		fireEvent.click(
			await screen.findByRole("checkbox", { name: /Ready session/ }),
		);
		fireEvent.click(screen.getByRole("checkbox", { name: /Remote session/ }));
		fireEvent.click(
			screen.getByRole("button", { name: "Import selected conversations" }),
		);
		fireEvent.click(screen.getByRole("button", { name: "Confirm import" }));
		fireEvent.click(
			await screen.findByRole("button", {
				name: "Retry unavailable conversations",
			}),
		);
		await waitFor(() =>
			expect(api.importConversations).toHaveBeenCalledTimes(2),
		);
		expect(api.importConversations).toHaveBeenNthCalledWith(
			2,
			"default",
			"work",
			["remote"],
		);
		expect(await screen.findByText("Imported: Remote session")).toBeVisible();
	});

	// --- Edge case 3: unavailable origin/session --------------------------------------------------------

	it("EC3 shows unavailable sessions and origins with a reason, never imports them, and keeps the switch available", async () => {
		twoAccounts();
		api.listConversations.mockResolvedValue({
			sessions: [
				{
					id: "local-ok",
					title: "Local session",
					origin: "Local",
					available: true,
					reason: null,
				},
				{
					id: "remote-down",
					title: "Remote session",
					origin: "SSH: builder",
					available: false,
					reason: "Host did not answer",
				},
			],
			unavailableOrigins: ["builder"],
		});
		renderQuotas();
		await screen.findAllByText(/latest collection/i);
		await enableImport();
		fireEvent.click(screen.getByRole("button", { name: "Find conversations" }));
		expect(
			await screen.findByText(/Unavailable: Host did not answer/),
		).toBeVisible();
		expect(screen.getByText("builder")).toBeVisible();
		const remote = screen.getByRole("checkbox", { name: /Remote session/ });
		expect(remote).toBeDisabled();
		fireEvent.click(screen.getByRole("checkbox", { name: /Local session/ }));
		fireEvent.click(
			screen.getByRole("button", { name: "Import selected conversations" }),
		);
		fireEvent.click(screen.getByRole("button", { name: "Confirm import" }));
		await waitFor(() =>
			expect(api.importConversations).toHaveBeenCalledTimes(1),
		);
		expect(api.importConversations).toHaveBeenCalledWith("default", "work", [
			"local-ok",
		]);
		expect(
			screen.getByRole("button", { name: "Open destination account" }),
		).toBeVisible();
	});

	it("EC3 keeps the switch available when listing fails and offers to retry", async () => {
		twoAccounts();
		api.listConversations.mockRejectedValue("desktop_running");
		renderQuotas();
		await screen.findAllByText(/latest collection/i);
		await enableImport();
		fireEvent.click(screen.getByRole("button", { name: "Find conversations" }));
		expect(await screen.findByRole("alert")).toHaveTextContent(
			/could not list/i,
		);
		const open = screen.getByRole("button", {
			name: "Open destination account",
		});
		expect(open).toBeVisible();
		expect(api.importConversations).not.toHaveBeenCalled();
		fireEvent.click(open);
		await waitFor(() => expect(api.openProfile).toHaveBeenCalledWith("work"));
	});

	// --- Edge case 4: already existing / deleted / newer in the destination -------------------------------

	it("EC4 shows skipped sessions with their reason and does not import them again", async () => {
		twoAccounts();
		api.listConversations.mockResolvedValue({
			sessions: [
				{
					id: "existing",
					title: "Existing session",
					origin: "Local",
					available: true,
					reason: null,
				},
			],
			unavailableOrigins: [],
		});
		api.importConversations.mockResolvedValue([
			{
				id: "existing",
				status: "skipped",
				reason: "The destination already has this conversation.",
			},
		]);
		renderQuotas();
		await screen.findAllByText(/latest collection/i);
		await enableImport();
		fireEvent.click(screen.getByRole("button", { name: "Find conversations" }));
		fireEvent.click(
			await screen.findByRole("checkbox", { name: /Existing session/ }),
		);
		fireEvent.click(
			screen.getByRole("button", { name: "Import selected conversations" }),
		);
		fireEvent.click(screen.getByRole("button", { name: "Confirm import" }));
		expect(
			await screen.findByText(
				/Skipped: Existing session — The destination already has this conversation/,
			),
		).toBeVisible();
		expect(
			screen.queryByRole("button", {
				name: "Retry unavailable conversations",
			}),
		).toBeNull();
	});

	// --- Edge case 5: individual failure --------------------------------------------------------------------

	it("EC5 shows an individual failure and retries only that session", async () => {
		twoAccounts();
		api.listConversations.mockResolvedValue({
			sessions: [
				{
					id: "ok",
					title: "Good session",
					origin: "Local",
					available: true,
					reason: null,
				},
				{
					id: "broken",
					title: "Broken session",
					origin: "Local",
					available: true,
					reason: null,
				},
			],
			unavailableOrigins: [],
		});
		api.importConversations
			.mockResolvedValueOnce([
				{ id: "ok", status: "imported", reason: null },
				{
					id: "broken",
					status: "unavailable",
					reason: "The conversation could not be written.",
				},
			])
			.mockResolvedValueOnce([
				{ id: "broken", status: "imported", reason: null },
			]);
		renderQuotas();
		await screen.findAllByText(/latest collection/i);
		await enableImport();
		fireEvent.click(screen.getByRole("button", { name: "Find conversations" }));
		fireEvent.click(
			await screen.findByRole("checkbox", { name: /Good session/ }),
		);
		fireEvent.click(screen.getByRole("checkbox", { name: /Broken session/ }));
		fireEvent.click(
			screen.getByRole("button", { name: "Import selected conversations" }),
		);
		fireEvent.click(screen.getByRole("button", { name: "Confirm import" }));
		expect(
			await screen.findByText(
				/Unavailable: Broken session — The conversation could not be written/,
			),
		).toBeVisible();
		fireEvent.click(
			screen.getByRole("button", { name: "Retry unavailable conversations" }),
		);
		await waitFor(() =>
			expect(api.importConversations).toHaveBeenCalledTimes(2),
		);
		expect(api.importConversations).toHaveBeenLastCalledWith(
			"default",
			"work",
			["broken"],
		);
	});

	it("EC5 keeps the switch available when the whole import call fails", async () => {
		twoAccounts();
		api.listConversations.mockResolvedValue({
			sessions: [
				{
					id: "one",
					title: "Only session",
					origin: "Local",
					available: true,
					reason: null,
				},
			],
			unavailableOrigins: [],
		});
		api.importConversations.mockRejectedValue("storage_unavailable");
		renderQuotas();
		await screen.findAllByText(/latest collection/i);
		await enableImport();
		fireEvent.click(screen.getByRole("button", { name: "Find conversations" }));
		fireEvent.click(
			await screen.findByRole("checkbox", { name: /Only session/ }),
		);
		fireEvent.click(
			screen.getByRole("button", { name: "Import selected conversations" }),
		);
		fireEvent.click(screen.getByRole("button", { name: "Confirm import" }));
		expect(await screen.findByRole("alert")).toHaveTextContent(
			/could not finish/i,
		);
		expect(
			screen.getByRole("button", { name: "Open destination account" }),
		).toBeVisible();
		fireEvent.click(
			screen.getByRole("button", { name: "Retry unavailable conversations" }),
		);
		await waitFor(() =>
			expect(api.importConversations).toHaveBeenCalledTimes(2),
		);
		expect(api.importConversations).toHaveBeenLastCalledWith(
			"default",
			"work",
			["one"],
		);
	});

	// --- Edge case 6: import disabled or cancelled ---------------------------------------------------------

	it("EC6 leaves the import off by default: no listing, no import, and the plain switch works", async () => {
		twoAccounts();
		renderQuotas();
		await screen.findAllByText(/latest collection/i);
		expect(api.listConversations).not.toHaveBeenCalled();
		expect(api.importConversations).not.toHaveBeenCalled();
		expect(
			screen.queryByRole("button", { name: "Find conversations" }),
		).toBeNull();
		fireEvent.click(await screen.findByRole("button", { name: "Open Work" }));
		await waitFor(() => expect(api.openProfile).toHaveBeenCalledWith("work"));
	});
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
