import { defaultKeymap, history, historyKeymap } from "@codemirror/commands";
import { json } from "@codemirror/lang-json";
import { markdown } from "@codemirror/lang-markdown";
import { MergeView } from "@codemirror/merge";
import {
	Annotation,
	Compartment,
	EditorState,
	type Extension,
	Transaction,
} from "@codemirror/state";
import { EditorView, keymap, lineNumbers } from "@codemirror/view";
import { useEffect, useRef, useState } from "react";
import {
	redactSecrets,
	type SecretSpan,
	secretField,
	setSecretsVisible,
} from "@/lib/editor-secrets";
import { editorHighlighting, editorTheme } from "@/lib/editor-theme";
import { cn } from "@/lib/utils";

type EditorLanguage = "json" | "toml" | "markdown";
const EMPTY_SPANS: SecretSpan[] = [];
const externalUpdate = Annotation.define<boolean>();

function languageExtension(language: EditorLanguage): Extension {
	return language === "json" ? json() : markdown();
}

interface CodeEditorProps {
	value: string;
	language?: EditorLanguage;
	readOnly?: boolean;
	onChange?: (nextValue: string, maskedValue: string) => void;
	className?: string;
	ariaLabel?: string;
	secretSpans?: SecretSpan[];
	revealSecrets?: boolean;
}

export function CodeEditor({
	value,
	language = "json",
	readOnly = false,
	onChange,
	className,
	ariaLabel = "Configuration editor",
	secretSpans = EMPTY_SPANS,
	revealSecrets = false,
}: CodeEditorProps) {
	const containerRef = useRef<HTMLDivElement | null>(null);
	const viewRef = useRef<EditorView | null>(null);
	const onChangeRef = useRef(onChange);
	const valueRef = useRef(value);
	const revealRef = useRef(revealSecrets);
	const readOnlyRef = useRef(readOnly);
	const [readOnlyCompartment] = useState(() => new Compartment());
	readOnlyRef.current = readOnly;
	revealRef.current = revealSecrets;
	valueRef.current = value;

	useEffect(() => {
		onChangeRef.current = onChange;
	}, [onChange]);

	useEffect(() => {
		const parent = containerRef.current;
		if (!parent) {
			return;
		}

		const secrets = secretField(
			valueRef.current,
			secretSpans,
			revealRef.current,
		);
		const extensions: Extension[] = [
			lineNumbers(),
			secrets,
			history(),
			editorHighlighting,
			keymap.of([...defaultKeymap, ...historyKeymap]),
			languageExtension(language),
			editorTheme,
			EditorView.contentAttributes.of({ "aria-label": ariaLabel }),
			readOnlyCompartment.of(EditorState.readOnly.of(readOnlyRef.current)),
			EditorView.updateListener.of((update) => {
				if (
					update.docChanged &&
					update.transactions.some(
						(transaction) => !transaction.annotation(externalUpdate),
					)
				) {
					const content = update.state.doc.toString();
					onChangeRef.current?.(
						content,
						redactSecrets(content, update.state.field(secrets).ranges),
					);
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
	}, [ariaLabel, readOnlyCompartment, language, secretSpans]);

	useEffect(() => {
		viewRef.current?.dispatch({
			effects: readOnlyCompartment.reconfigure(
				EditorState.readOnly.of(readOnly),
			),
		});
	}, [readOnlyCompartment, readOnly]);

	useEffect(() => {
		viewRef.current?.dispatch({ effects: setSecretsVisible.of(revealSecrets) });
	}, [revealSecrets]);

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
			editorHighlighting,
			languageExtension(language),
			editorTheme,
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
