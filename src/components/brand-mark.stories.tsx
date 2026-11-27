import type { Meta, StoryObj } from "@storybook/react-vite";
import { BrandMark } from "./brand-mark";

const meta: Meta<typeof BrandMark> = {
	title: "Components/BrandMark",
	component: BrandMark,
};

export default meta;
type Story = StoryObj<typeof BrandMark>;

export const Badge: Story = {
	args: {
		variant: "badge",
		className: "size-12",
	},
};

export const Glyph: Story = {
	args: {
		variant: "glyph",
		className: "size-8 text-primary",
	},
};
