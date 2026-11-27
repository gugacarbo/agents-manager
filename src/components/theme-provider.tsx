import {
	createContext,
	type ReactNode,
	useCallback,
	useContext,
	useEffect,
	useMemo,
	useState,
} from "react";

export type Theme = "dark" | "light";

interface ThemeContextValue {
	theme: Theme;
	setTheme: (theme: Theme) => void;
	toggleTheme: () => void;
}

const ThemeContext = createContext<ThemeContextValue | null>(null);

interface ThemeProviderProps {
	children: ReactNode;
	defaultTheme?: Theme;
	storageKey?: string;
}

function readStoredTheme(storageKey: string, fallback: Theme): Theme {
	try {
		const stored = window.localStorage.getItem(storageKey);
		if (stored === "dark" || stored === "light") {
			return stored;
		}
	} catch {
		// Ignore storage access failures in restricted environments.
	}
	return fallback;
}

export function ThemeProvider({
	children,
	defaultTheme = "dark",
	storageKey = "agents-manager-theme",
}: ThemeProviderProps) {
	const [theme, setThemeState] = useState<Theme>(() =>
		readStoredTheme(storageKey, defaultTheme),
	);

	useEffect(() => {
		document.documentElement.classList.toggle("dark", theme === "dark");
		try {
			window.localStorage.setItem(storageKey, theme);
		} catch {
			// Ignore storage write failures in restricted environments.
		}
	}, [storageKey, theme]);

	const setTheme = useCallback((nextTheme: Theme) => {
		setThemeState(nextTheme);
	}, []);

	const toggleTheme = useCallback(() => {
		setThemeState((current) => (current === "dark" ? "light" : "dark"));
	}, []);

	const value = useMemo(
		() => ({ theme, setTheme, toggleTheme }),
		[setTheme, theme, toggleTheme],
	);

	return (
		<ThemeContext.Provider value={value}>{children}</ThemeContext.Provider>
	);
}

export function useTheme(): ThemeContextValue {
	const context = useContext(ThemeContext);
	if (!context) {
		throw new Error("useTheme must be used within a ThemeProvider");
	}
	return context;
}

if (import.meta.hot) {
	import.meta.hot.accept(() => {
		window.location.reload();
	});
}
