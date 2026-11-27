import type { ErrorComponentProps } from "@tanstack/react-router";
import { EmptyState, ErrorState } from "@/components/patterns";
import { Button } from "@/components/ui/button";

interface RouteNotFoundFallbackProps {
	onGoHome?: () => void;
}

export function RouteNotFoundFallback({
	onGoHome,
}: RouteNotFoundFallbackProps) {
	function handleGoHome() {
		if (onGoHome) {
			onGoHome();
			return;
		}
		window.location.hash = "#/";
	}

	return (
		<EmptyState
			title="Page not found"
			description="The requested route does not exist in this workspace."
			action={
				<Button variant="outline" size="sm" onClick={handleGoHome}>
					Back to overview
				</Button>
			}
		/>
	);
}

interface RouteErrorFallbackProps {
	error?: ErrorComponentProps["error"];
	reset?: ErrorComponentProps["reset"];
}

export function RouteErrorFallback({ error, reset }: RouteErrorFallbackProps) {
	const message =
		error instanceof Error ? error.message : "An unexpected error occurred.";

	return (
		<ErrorState
			title="Failed to render route"
			message={message}
			action={
				<Button
					variant="outline"
					size="sm"
					onClick={() => {
						if (reset) {
							reset();
							return;
						}
						window.location.reload();
					}}
				>
					Try again
				</Button>
			}
		/>
	);
}
