import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { createMemoryHistory, RouterProvider } from "@tanstack/react-router";
import { clearMocks, mockIPC as tauriMockIPC } from "@tauri-apps/api/mocks";
import {
	cleanup,
	fireEvent,
	render,
	screen,
	within,
} from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import type { AccountStatus_Serialize, ClaudeAccount } from "@/bindings/tauri";
import { createAppRouter } from "@/router";

const mockIPC: typeof tauriMockIPC = (callback, options) =>
	tauriMockIPC(
		(command, args) =>
			command === "list_claude_desktop_quota_profiles"
				? []
				: callback(command, args),
		options,
	);

const personal = { id: "profile-personal", displayName: "Personal" };
const work = { id: "profile-work", displayName: "Work" };

function deferred<T>() {
	let resolve!: (value: T) => void;
	const promise = new Promise<T>((done) => {
		resolve = done;
	});
	return { promise, resolve };
}

function renderPage(path = "/accounts/claude") {
	const router = createAppRouter(
		createMemoryHistory({ initialEntries: [path] }),
	);
	const client = new QueryClient();
	return render(
		<QueryClientProvider client={client}>
			<RouterProvider router={router} />
		</QueryClientProvider>,
	);
}

async function profile(name: string) {
	return within(await screen.findByRole("listitem", { name }));
}

async function createProfile(name: string) {
	fireEvent.change(
		await screen.findByRole("textbox", { name: "Display name" }),
		{
			target: { value: name },
		},
	);
	fireEvent.click(screen.getByRole("button", { name: "Add profile" }));
	return profile(name);
}

function profileId(args: unknown) {
	const { profileId } = args as { profileId: string };
	return profileId;
}

afterEach(() => {
	cleanup();
	clearMocks();
	delete (window as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;
});

describe("Claude Code account profiles", () => {
	it("1: saves separate profiles and rechecks their independent connection states after reopening", async () => {
		const accounts: ClaudeAccount[] = [];
		const connected = new Set<string>();
		const listed = deferred<ClaudeAccount[]>();
		mockIPC((command, args) => {
			switch (command) {
				case "list_claude_accounts":
					return listed.promise.then(() => [...accounts]);
				case "create_claude_account": {
					const { displayName } = args as { displayName: string };
					const account = {
						id: `profile-${displayName.toLowerCase()}`,
						displayName,
					};
					accounts.push(account);
					return account;
				}
				case "start_claude_account_login":
					connected.add(profileId(args));
					return { profileId: profileId(args), state: "connected" };
				case "get_claude_account_status":
					return {
						profileId: profileId(args),
						state: connected.has(profileId(args))
							? "connected"
							: "disconnected",
					};
				case "list_agents":
				case "list_ssh_environments":
					return [];
				default:
					throw new Error(`Unexpected IPC: ${command}`);
			}
		});
		const first = renderPage("/");
		fireEvent.click(
			await screen.findByRole("link", { name: "Claude Code accounts" }),
		);
		expect(
			await screen.findByRole("heading", { name: "Claude Code accounts" }),
		).toBeVisible();
		expect(screen.getByText(/does not use Claude Desktop/i)).toBeVisible();
		expect(
			screen.queryByRole("combobox", { name: "Environment" }),
		).not.toBeInTheDocument();
		expect(screen.getByRole("status")).toHaveTextContent(/loading profiles/i);
		listed.resolve(accounts);
		for (const name of ["Personal", "Work"]) {
			const row = await createProfile(name);
			await row.findByText("Disconnected");
			fireEvent.click(row.getByRole("button", { name: "Start login" }));
			expect(await row.findByText("Connected")).toBeVisible();
		}
		first.unmount();
		connected.delete(work.id);
		renderPage();
		expect(
			await (await profile("Personal")).findByText("Connected"),
		).toBeVisible();
		expect(
			await (await profile("Work")).findByText("Disconnected"),
		).toBeVisible();
	});

	it("2: allows another official login after an unconfirmed attempt", async () => {
		let attempts = 0;
		mockIPC((command, args) => {
			if (command === "list_claude_accounts") return [personal];
			if (command === "get_claude_account_status")
				return { profileId: profileId(args), state: "disconnected" };
			if (command === "start_claude_account_login") {
				attempts++;
				return {
					profileId: profileId(args),
					state: attempts === 1 ? "disconnected" : "connected",
				};
			}
		});
		renderPage();
		expect(
			await screen.findByText(/official Claude Code login/i),
		).toBeVisible();
		const row = await profile("Personal");
		await row.findByText("Disconnected");
		fireEvent.click(row.getByRole("button", { name: "Start login" }));
		expect(
			await row.findByRole("button", { name: "Retry login" }),
		).toBeEnabled();
		expect(row.queryByText("Connected")).not.toBeInTheDocument();
		fireEvent.click(row.getByRole("button", { name: "Retry login" }));
		expect(await row.findByText("Connected")).toBeVisible();
	});

	it("3: shows CLI prerequisites and lets the user retry verification", async () => {
		let fixed = false;
		mockIPC((command, args) => {
			if (command === "list_claude_accounts") return [personal];
			if (command === "get_claude_account_status") {
				return fixed
					? { profileId: profileId(args), state: "disconnected" }
					: {
							profileId: profileId(args),
							state: "verificationFailed",
							errorCode: "cliUnavailable",
						};
			}
		});
		renderPage();
		const row = await profile("Personal");
		expect(await row.findByText("Verification failed")).toBeVisible();
		expect(row.getByRole("alert")).toHaveTextContent(
			/install or update Claude Code.+2\.1\.268/i,
		);
		expect(row.queryByText("Disconnected")).not.toBeInTheDocument();
		fixed = true;
		fireEvent.click(row.getByRole("button", { name: "Retry status check" }));
		expect(await row.findByText("Disconnected")).toBeVisible();
	});

	it("4: blocks duplicate login and resumes status checks when revisiting a pending profile", async () => {
		const finished = deferred<AccountStatus_Serialize>();
		let loginStarts = 0;
		let loginComplete = false;
		mockIPC((command, args) => {
			if (command === "list_claude_accounts") return [personal];
			if (command === "get_claude_account_status") {
				return {
					profileId: profileId(args),
					state: loginComplete
						? "connected"
						: loginStarts
							? "pending"
							: "disconnected",
				};
			}
			if (command === "start_claude_account_login") {
				loginStarts++;
				return finished.promise;
			}
		});
		const first = renderPage();
		const row = await profile("Personal");
		await row.findByText("Disconnected");
		const start = row.getByRole("button", { name: "Start login" });
		fireEvent.click(start);
		fireEvent.click(start);
		expect(await row.findByRole("status")).toHaveTextContent(
			"Login in progress",
		);
		expect(
			row.getByRole("button", { name: "Login in progress" }),
		).toBeDisabled();
		expect(loginStarts).toBe(1);
		first.unmount();
		renderPage();
		const reopened = await profile("Personal");
		expect(
			await reopened.findByRole("button", { name: "Login in progress" }),
		).toBeDisabled();
		expect(reopened.getByRole("status")).toHaveTextContent("Login in progress");
		loginComplete = true;
		finished.resolve({ profileId: personal.id, state: "connected" });
		expect(await reopened.findByText("Connected")).toBeVisible();
		expect(loginStarts).toBe(1);
	});

	it("5: hides unrecognized response fields and raw IPC rejection details", async () => {
		mockIPC((command, args) => {
			if (command === "list_claude_accounts")
				return [{ ...personal, token: "fake-account-secret" }];
			if (command === "get_claude_account_status")
				return {
					profileId: profileId(args),
					state: "disconnected",
					email: "private@example.test",
				};
			if (command === "start_claude_account_login") throw "fake-invoke-secret";
		});
		renderPage();
		const row = await profile("Personal");
		await row.findByText("Disconnected");
		fireEvent.click(row.getByRole("button", { name: "Start login" }));
		expect(await row.findByRole("alert")).toHaveTextContent(/try again/i);
		expect(
			screen.queryByText(
				/fake-account-secret|private@example\.test|fake-invoke-secret/,
			),
		).not.toBeInTheDocument();
	});
});
