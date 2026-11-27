import { defineConfig, devices } from "@playwright/test";

const devServerHost = "127.0.0.1";
const devServerUrl = `http://${devServerHost}:1420`;

export default defineConfig({
	testDir: "./tests/e2e",
	fullyParallel: true,
	forbidOnly: Boolean(process.env.CI),
	retries: process.env.CI ? 2 : 0,
	reporter: process.env.CI ? "github" : "list",
	use: {
		baseURL: process.env.PLAYWRIGHT_BASE_URL ?? devServerUrl,
		trace: "on-first-retry",
	},
	projects: [
		{
			name: "chromium",
			use: { ...devices["Desktop Chrome"] },
		},
	],
	webServer: {
		command: `bun run dev:web --host ${devServerHost}`,
		url: devServerUrl,
		reuseExistingServer: !process.env.CI,
	},
});
