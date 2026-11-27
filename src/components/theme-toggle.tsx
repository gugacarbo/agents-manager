import { Moon, Sun } from "lucide-react";
import { useTheme } from "@/components/theme-provider";
import { Button } from "@/components/ui/button";

export function ThemeToggle() {
	const { theme, toggleTheme } = useTheme();
	const isDark = theme === "dark";
	const label = isDark ? "Switch to light mode" : "Switch to dark mode";

	return (
		<Button
			variant="outline"
			size="sm"
			onClick={toggleTheme}
			aria-label={label}
			title={label}
		>
			{isDark ? (
				<Sun className="size-4" aria-hidden="true" />
			) : (
				<Moon className="size-4" aria-hidden="true" />
			)}
		</Button>
	);
}
