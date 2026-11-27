import {
	defaultHighlightStyle,
	HighlightStyle,
	syntaxHighlighting,
} from "@codemirror/language";
import { EditorView } from "@codemirror/view";

export const editorHighlighting = syntaxHighlighting(
	HighlightStyle.define(
		defaultHighlightStyle.specs.map((spec) => {
			const { color } = spec;
			return typeof color === "string"
				? {
						...spec,
						color: `light-dark(${color}, color-mix(in srgb, ${color} 28%, var(--foreground)))`,
					}
				: spec;
		}),
	),
);
export const editorTheme = EditorView.theme({
	"&": {
		backgroundColor: "var(--background)",
		color: "var(--foreground)",
		fontSize: "13px",
		height: "100%",
	},
	".cm-scroller": {
		fontFamily: "var(--font-mono)",
	},
	".cm-gutters": {
		backgroundColor: "var(--muted)",
		color: "var(--muted-foreground)",
		borderRight: "1px solid var(--border)",
	},
	".cm-activeLine": {
		backgroundColor: "var(--accent)",
	},
	".cm-cursor": {
		borderLeftColor: "var(--foreground)",
	},
	".cm-content ::selection": {
		backgroundColor: "var(--selection)",
		color: "var(--selection-foreground)",
	},
});
