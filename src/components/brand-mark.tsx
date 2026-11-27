import type { SVGProps } from "react";

type BrandMarkProps = SVGProps<SVGSVGElement> & {
	variant?: "badge" | "glyph";
};

const ROUNDED_HEX_BADGE =
	"M 230.02 119.00 Q 256.00 104.00 281.98 119.00 L 361.66 165.00 Q 387.64 180.00 387.64 210.00 L 387.64 302.00 Q 387.64 332.00 361.66 347.00 L 281.98 393.00 Q 256.00 408.00 230.02 393.00 L 150.34 347.00 Q 124.36 332.00 124.36 302.00 L 124.36 210.00 Q 124.36 180.00 150.34 165.00 Z";

const ROUNDED_HEX_GLYPH =
	"M 219.63 71.00 Q 256.00 50.00 292.37 71.00 L 398.03 132.00 Q 434.40 153.00 434.40 195.00 L 434.40 317.00 Q 434.40 359.00 398.03 380.00 L 292.37 441.00 Q 256.00 462.00 219.63 441.00 L 113.97 380.00 Q 77.60 359.00 77.60 317.00 L 77.60 195.00 Q 77.60 153.00 113.97 132.00 Z";

export function BrandMark({ variant = "badge", ...props }: BrandMarkProps) {
	if (variant === "glyph") {
		return (
			<svg
				xmlns="http://www.w3.org/2000/svg"
				viewBox="0 0 512 512"
				aria-hidden="true"
				{...props}
			>
				<path
					d={ROUNDED_HEX_GLYPH}
					fill="none"
					stroke="currentColor"
					strokeWidth="32"
					strokeLinejoin="round"
					strokeLinecap="round"
					opacity="0.55"
				/>
				<g stroke="currentColor" strokeWidth="36" strokeLinecap="round">
					<line x1="256" y1="256" x2="256" y2="60.5" />
					<line x1="256" y1="256" x2="425.31" y2="353.75" />
					<line x1="256" y1="256" x2="86.69" y2="353.75" />
				</g>
				<circle cx="256" cy="60.5" r="44" fill="currentColor" />
				<circle cx="425.31" cy="353.75" r="44" fill="currentColor" />
				<circle cx="86.69" cy="353.75" r="44" fill="currentColor" />
				<circle
					cx="256"
					cy="256"
					r="44"
					fill="none"
					stroke="currentColor"
					strokeWidth="36"
				/>
			</svg>
		);
	}

	return (
		<svg
			xmlns="http://www.w3.org/2000/svg"
			viewBox="0 0 512 512"
			aria-hidden="true"
			{...props}
		>
			<rect
				x="24"
				y="24"
				width="464"
				height="464"
				rx="112"
				fill="#171815"
				stroke="#36372F"
				strokeWidth="12"
			/>
			<path
				d={ROUNDED_HEX_BADGE}
				fill="none"
				stroke="#84500B"
				strokeWidth="22"
				strokeLinejoin="round"
				strokeLinecap="round"
			/>
			<g stroke="#EAC17B" strokeWidth="26" strokeLinecap="round">
				<line x1="256" y1="256" x2="256" y2="111.5" />
				<line x1="256" y1="256" x2="381.14" y2="328.25" />
				<line x1="256" y1="256" x2="130.86" y2="328.25" />
			</g>
			<circle cx="256" cy="111.5" r="32" fill="#EAC17B" />
			<circle cx="381.14" cy="328.25" r="32" fill="#EAC17B" />
			<circle cx="130.86" cy="328.25" r="32" fill="#EAC17B" />
			<circle cx="256" cy="256" r="46" fill="#EAC17B" />
			<circle cx="256" cy="256" r="18" fill="#171815" />
		</svg>
	);
}
