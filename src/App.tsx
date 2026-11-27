import { RouterProvider } from "@tanstack/react-router";
import { useMemo } from "react";
import { createAppRouter } from "./router";

export function App() {
	const router = useMemo(() => createAppRouter(), []);
	return <RouterProvider router={router} />;
}
