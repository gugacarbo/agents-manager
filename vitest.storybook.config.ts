import path from "node:path";
import { fileURLToPath } from "node:url";
import { storybookTest } from "@storybook/addon-vitest/vitest-plugin";
import tailwindcss from "@tailwindcss/vite";
import { playwright } from "@vitest/browser-playwright";
import { defineConfig } from "vitest/config";

const dirname = path.dirname(fileURLToPath(import.meta.url));

export default defineConfig({
	plugins: [
		tailwindcss(),
		storybookTest({ configDir: path.join(dirname, ".storybook") }),
	],
	optimizeDeps: {
		include: ["@tauri-apps/api/mocks", "@tauri-apps/api/window", "sonner"],
	},
	resolve: {
		alias: {
			"@": path.join(dirname, "src"),
		},
	},
	test: {
		name: "storybook",
		browser: {
			enabled: true,
			headless: true,
			provider: playwright({}),
			instances: [{ browser: "chromium" }],
		},
	},
});
