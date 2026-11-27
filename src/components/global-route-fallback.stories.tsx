import type { Meta, StoryObj } from "@storybook/react-vite";
import {
	RouteErrorFallback,
	RouteNotFoundFallback,
} from "./global-route-fallback";

const meta: Meta<typeof RouteNotFoundFallback> = {
	title: "Components/GlobalRouteFallback",
	component: RouteNotFoundFallback,
};

export default meta;

export const NotFound: StoryObj<typeof RouteNotFoundFallback> = {
	render: () => <RouteNotFoundFallback onGoHome={() => {}} />,
};

export const RouteError: StoryObj<typeof RouteErrorFallback> = {
	render: () => (
		<RouteErrorFallback
			error={new Error("IPC channel unavailable")}
			reset={() => {}}
		/>
	),
};
