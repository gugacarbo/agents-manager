import { RouterProvider } from "@tanstack/react-router";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useEffect, useMemo } from "react";
import { isTauriRuntime } from "./lib/agents";
import { createAppRouter } from "./router";

function useDesktopWindowCorners() {
	useEffect(() => {
		if (!isTauriRuntime()) return;

		const root = document.documentElement;
		const roundedClass = "desktop-window-rounded";
		let revision = 0;

		const syncCorners = async () => {
			const current = ++revision;
			try {
				const appWindow = getCurrentWindow();
				const [maximized, fullscreen] = await Promise.all([
					appWindow.isMaximized(),
					appWindow.isFullscreen(),
				]);
				if (current === revision) {
					root.classList.toggle(
						roundedClass,
						maximized === false && fullscreen === false,
					);
				}
			} catch {
				if (current === revision) root.classList.remove(roundedClass);
			}
		};

		const update = () => {
			void syncCorners();
		};
		window.addEventListener("resize", update);
		window.addEventListener("focus", update);
		update();

		return () => {
			++revision;
			window.removeEventListener("resize", update);
			window.removeEventListener("focus", update);
			root.classList.remove(roundedClass);
		};
	}, []);
}

export function App() {
	useDesktopWindowCorners();
	const router = useMemo(() => createAppRouter(), []);
	return <RouterProvider router={router} />;
}
