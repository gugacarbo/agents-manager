import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Moon, PanelBottomClose, SlidersHorizontal, Sun } from "lucide-react";
import { toast } from "sonner";
import { useTheme } from "@/components/theme-provider";
import { Button } from "@/components/ui/button";
import {
	DropdownMenu,
	DropdownMenuCheckboxItem,
	DropdownMenuContent,
	DropdownMenuItem,
	DropdownMenuLabel,
	DropdownMenuSeparator,
	DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import {
	appPreferencesQueryKey,
	getAppPreferences,
	setAppPreferences,
} from "@/lib/preferences";

interface AppPreferencesMenuProps {
	showTrayOption: boolean;
}

export function AppPreferencesMenu({
	showTrayOption,
}: AppPreferencesMenuProps) {
	const { theme, toggleTheme } = useTheme();
	const isDark = theme === "dark";

	const queryClient = useQueryClient();
	const { data } = useQuery({
		queryKey: appPreferencesQueryKey,
		queryFn: getAppPreferences,
		enabled: showTrayOption,
	});
	const mutation = useMutation({
		mutationFn: setAppPreferences,
		onSuccess: (saved) => {
			queryClient.setQueryData(appPreferencesQueryKey, saved);
			toast.success(
				saved.minimizeToTray
					? "Closing the window now keeps the app in the system tray."
					: "Closing the window will now quit the app.",
			);
		},
		onError: () => {
			toast.error("Could not update tray behavior. Try again.");
		},
	});

	const minimizeToTray = data?.minimizeToTray ?? true;

	return (
		<DropdownMenu>
			<DropdownMenuTrigger asChild>
				<Button
					variant="ghost"
					size="sm"
					aria-label="App preferences"
					title="App preferences"
				>
					<SlidersHorizontal className="size-4" aria-hidden="true" />
				</Button>
			</DropdownMenuTrigger>
			<DropdownMenuContent
				align="end"
				aria-label="App preferences"
				className="w-60"
			>
				<DropdownMenuLabel>Preferences</DropdownMenuLabel>
				<DropdownMenuSeparator />
				<DropdownMenuItem
					className="justify-between"
					onSelect={(event) => {
						event.preventDefault();
						toggleTheme();
					}}
				>
					<span className="flex items-center gap-2">
						{isDark ? (
							<Sun className="size-4" aria-hidden="true" />
						) : (
							<Moon className="size-4" aria-hidden="true" />
						)}
						<span>Theme</span>
					</span>
					<span className="rounded bg-muted px-1.5 py-0.5 text-xs text-muted-foreground">
						{isDark ? "Dark" : "Light"}
					</span>
				</DropdownMenuItem>
				{showTrayOption ? (
					<DropdownMenuCheckboxItem
						checked={minimizeToTray}
						disabled={mutation.isPending}
						onSelect={(event) => {
							event.preventDefault();
						}}
						onCheckedChange={(checked) => {
							mutation.mutate({ minimizeToTray: Boolean(checked) });
						}}
					>
						<PanelBottomClose className="size-4" aria-hidden="true" />
						<span>Run in background</span>
					</DropdownMenuCheckboxItem>
				) : null}
			</DropdownMenuContent>
		</DropdownMenu>
	);
}
