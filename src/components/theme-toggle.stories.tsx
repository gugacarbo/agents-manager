import type { Meta, StoryObj } from "@storybook/react-vite";
import { ThemeProvider } from "./theme-provider";
import { ThemeToggle } from "./theme-toggle";

const meta: Meta<typeof ThemeToggle> = {
	title: "Components/ThemeToggle",
	component: ThemeToggle,
};

export default meta;
type Story = StoryObj<typeof ThemeToggle>;

export const DarkMode: Story = {
	render: () => (
		<ThemeProvider defaultTheme="dark" storageKey="storybook-theme-dark">
			<ThemeToggle />
		</ThemeProvider>
	),
};

export const LightMode: Story = {
	render: () => (
		<ThemeProvider defaultTheme="light" storageKey="storybook-theme-light">
			<ThemeToggle />
		</ThemeProvider>
	),
};
