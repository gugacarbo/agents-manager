import { Toaster as SonnerToaster, type ToasterProps } from "sonner";

export function Toaster(props: ToasterProps) {
	return (
		<SonnerToaster
			theme="dark"
			position="bottom-right"
			richColors
			toastOptions={{
				className:
					"border border-border bg-card text-card-foreground shadow-lg",
			}}
			{...props}
		/>
	);
}
