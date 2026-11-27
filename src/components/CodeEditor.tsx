import { defaultKeymap, history, historyKeymap } from "@codemirror/commands";
import { json } from "@codemirror/lang-json";
import { markdown } from "@codemirror/lang-markdown";
import {
	defaultHighlightStyle,
	syntaxHighlighting,
} from "@codemirror/language";
import { MergeView } from "@codemirror/merge";
import {
	Annotation,
	EditorState,
	type Extension,
	Transaction,
} from "@codemirror/state";
import { EditorView, keymap, lineNumbers } from "@codemirror/view";
import { useEffect, useRef } from "react";
import { cn } from "@/lib/utils";

type EditorLanguage = "json" | "markdown";
const externalUpdate = Annotation.define<boolean>();

const darkEditorTheme = EditorView.theme(
	{
		"&": {
			backgroundColor: "#0f172a",
			color: "#e2e8f0",
			fontSize: "13px",
			height: "100%",
		},
		".cm-scroller": {
			fontFamily:
				'ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", monospace',
		},
		".cm-gutters": {
			backgroundColor: "#020617",
			color: "#64748b",
			borderRight: "1px solid #1e293b",
		},
		".cm-activeLine": {
			backgroundColor: "rgba(30, 41, 59, 0.45)",
		},
		".cm-cursor": {
			borderLeftColor: "#f8fafc",
		},
	},
	{ dark: true },
);

function languageExtension(language: EditorLanguage): Extension {
	return language === "json" ? json() : markdown();
}

interface CodeEditorProps {
	value: string;
	language?: EditorLanguage;
	readOnly?: boolean;
	onChange?: (nextValue: string) => void;
	className?: string;
	ariaLabel?: string;
}

export function CodeEditor({
	value,
	language = "json",
	readOnly = false,
	onChange,
	className,
	ariaLabel = "Configuration editor",
}: CodeEditorProps) {
	const containerRef = useRef<HTMLDivElement | null>(null);
	const viewRef = useRef<EditorView | null>(null);
	const onChangeRef = useRef(onChange);
	const valueRef = useRef(value);
	valueRef.current = value;

	useEffect(() => {
		onChangeRef.current = onChange;
	}, [onChange]);

	useEffect(() => {
		const parent = containerRef.current;
		if (!parent) {
			return;
		}

		const extensions: Extension[] = [
			lineNumbers(),
			history(),
			syntaxHighlighting(defaultHighlightStyle),
			keymap.of([...defaultKeymap, ...historyKeymap]),
			languageExtension(language),
			darkEditorTheme,
			EditorView.contentAttributes.of({ "aria-label": ariaLabel }),
			EditorState.readOnly.of(readOnly),
			EditorView.updateListener.of((update) => {
				if (
					update.docChanged &&
					update.transactions.some(
						(transaction) => !transaction.annotation(externalUpdate),
					)
				) {
					onChangeRef.current?.(update.state.doc.toString());
				}
			}),
		];

		const view = new EditorView({
			state: EditorState.create({
				doc: valueRef.current,
				extensions,
			}),
			parent,
		});

		viewRef.current = view;

		return () => {
			view.destroy();
			viewRef.current = null;
		};
	}, [ariaLabel, language, readOnly]);

	useEffect(() => {
		const view = viewRef.current;
		if (view && view.state.doc.toString() !== value) {
			view.dispatch({
				changes: { from: 0, to: view.state.doc.length, insert: value },
				annotations: [
					externalUpdate.of(true),
					Transaction.addToHistory.of(false),
				],
			});
		}
	}, [value]);

	return (
		<div
			ref={containerRef}
			className={cn(
				"min-h-64 overflow-hidden rounded-lg border border-border bg-card",
				className,
			)}
		/>
	);
}

interface DiffViewerProps {
	original: string;
	modified: string;
	language?: EditorLanguage;
	className?: string;
}

export function DiffViewer({
	original,
	modified,
	language = "json",
	className,
}: DiffViewerProps) {
	const containerRef = useRef<HTMLDivElement | null>(null);

	useEffect(() => {
		const parent = containerRef.current;
		if (!parent) {
			return;
		}

		const sharedExtensions: Extension[] = [
			lineNumbers(),
			syntaxHighlighting(defaultHighlightStyle),
			languageExtension(language),
			darkEditorTheme,
			EditorState.readOnly.of(true),
		];

		const mergeView = new MergeView({
			a: {
				doc: original,
				extensions: sharedExtensions,
			},
			b: {
				doc: modified,
				extensions: sharedExtensions,
			},
			parent,
			highlightChanges: true,
			gutter: true,
		});

		return () => {
			mergeView.destroy();
		};
	}, [language, modified, original]);

	return (
		<section
			ref={containerRef}
			aria-label="Configuration diff viewer"
			className={cn(
				"min-h-64 overflow-hidden rounded-lg border border-border bg-card",
				className,
			)}
		/>
	);
}
