import { StateEffect, StateField } from "@codemirror/state";
import { Decoration, EditorView, WidgetType } from "@codemirror/view";

export type SecretSpan = { startByte: number; endByte: number; kind: string };

class SecretWidget extends WidgetType {
	toDOM() {
		const span = document.createElement("span");
		span.textContent = "••••••";
		span.setAttribute("aria-label", "Hidden secret");
		return span;
	}
}

export function secretDecorations(content: string, spans: SecretSpan[]) {
	const encoder = new TextEncoder();
	const decoder = new TextDecoder();
	const bytes = encoder.encode(content);
	return Decoration.set(
		spans.flatMap(({ startByte, endByte }) => {
			const from = decoder.decode(bytes.slice(0, startByte)).length;
			const to = decoder.decode(bytes.slice(0, endByte)).length;
			return from < to && to <= content.length
				? [Decoration.replace({ widget: new SecretWidget() }).range(from, to)]
				: [];
		}),
		true,
	);
}

export function redactSecrets(
	content: string,
	ranges: ReturnType<typeof secretDecorations>,
) {
	const parts: string[] = [];
	let cursor = 0;
	ranges.between(0, content.length, (from, to) => {
		parts.push(content.slice(cursor, from), "••••••");
		cursor = to;
	});
	parts.push(content.slice(cursor));
	return parts.join("");
}

export function maskedConfig(content: string, spans: SecretSpan[]) {
	return redactSecrets(content, secretDecorations(content, spans));
}

export const setSecretsVisible = StateEffect.define<boolean>();

export function secretField(
	content: string,
	spans: SecretSpan[],
	visible: boolean,
) {
	return StateField.define({
		create: () => ({
			ranges: secretDecorations(content, spans),
			visible: visible,
		}),
		update: (state, transaction) => ({
			ranges: state.ranges.map(transaction.changes),
			visible: transaction.effects.reduce(
				(visible, effect) =>
					effect.is(setSecretsVisible) ? effect.value : visible,
				state.visible,
			),
		}),
		provide: (field) =>
			EditorView.decorations.from(field, (state) =>
				state.visible ? Decoration.none : state.ranges,
			),
	});
}
