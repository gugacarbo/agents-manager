import type { Meta, StoryObj } from "@storybook/react-vite";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { ThemeProvider } from "@/components/theme-provider";
import { type AppPreferences, appPreferencesQueryKey } from "@/lib/preferences";
import { AppPreferencesMenu } from "./app-preferences-menu";

function createStoryQueryClient(preferences: AppPreferences) {
	const queryClient = new QueryClient({
		defaultOptions: {
			queries: {
				retry: false,
				staleTime: Number.POSITIVE_INFINITY,
			},
		},
	});
	queryClient.setQueryData(appPreferencesQueryKey, preferences);
	return queryClient;
}

const meta: Meta<typeof AppPreferencesMenu> = {
	title: "Components/AppPreferencesMenu",
	component: AppPreferencesMenu,
};

export default meta;
type Story = StoryObj<typeof AppPreferencesMenu>;

export const Desktop: Story = {
	render: () => (
		<ThemeProvider defaultTheme="dark" storageKey="storybook-prefs-desktop">
			<QueryClientProvider
				client={createStoryQueryClient({ minimizeToTray: true })}
			>
				<div className="flex justify-end p-8">
					<AppPreferencesMenu showTrayOption />
				</div>
			</QueryClientProvider>
		</ThemeProvider>
	),
};

export const BrowserPreview: Story = {
	render: () => (
		<ThemeProvider defaultTheme="light" storageKey="storybook-prefs-web">
			<QueryClientProvider
				client={createStoryQueryClient({ minimizeToTray: false })}
			>
				<div className="flex justify-end p-8">
					<AppPreferencesMenu showTrayOption={false} />
				</div>
			</QueryClientProvider>
		</ThemeProvider>
	),
};
