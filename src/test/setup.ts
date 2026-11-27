import "@testing-library/jest-dom/vitest";

Object.defineProperty(window, "scrollTo", {
	value: () => {},
	writable: true,
});

if (typeof globalThis.ResizeObserver === "undefined") {
	globalThis.ResizeObserver = class ResizeObserver {
		observe() {}
		unobserve() {}
		disconnect() {}
	};
}
