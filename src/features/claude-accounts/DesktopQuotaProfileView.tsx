import { Button } from "@/components/ui/button";
import type {
	DesktopQuotaProfile_Serialize,
	DesktopQuotaWindow_Serialize,
} from "@/lib/claude-accounts";
import { type Display, message } from "./desktop-quota-display";

function time(value: string | null | undefined) {
	if (!value) return "Unavailable";
	const date = new Date(value);
	return Number.isNaN(date.getTime()) ? "Unavailable" : date.toLocaleString();
}
function WindowQuota({
	label,
	window,
}: {
	label: string;
	window: DesktopQuotaWindow_Serialize | null;
}) {
	return (
		<fieldset
			aria-label={label}
			className="space-y-1 rounded-md border border-border p-3"
		>
			<legend className="font-medium">{label}</legend>
			<p className="text-sm">
				Used:{" "}
				{window?.utilization == null ? (
					<span>Unavailable</span>
				) : (
					<span>{window.utilization}%</span>
				)}
			</p>
			<p className="text-sm">
				Renews: <span>{time(window?.resets_at)}</span>
			</p>
		</fieldset>
	);
}
export function DesktopQuotaProfileView({
	profile,
	isolated,
	isolatedStatus,
	status,
	switching,
	refresh,
	action,
}: {
	profile: DesktopQuotaProfile_Serialize;
	isolated: DesktopQuotaProfile_Serialize | undefined;
	isolatedStatus: Display | undefined;
	status: Display | undefined;
	switching: boolean;
	refresh: (profile: DesktopQuotaProfile_Serialize) => Promise<void>;
	action: (
		profile: DesktopQuotaProfile_Serialize,
		kind: "open" | "link" | "switch",
	) => Promise<void>;
}) {
	const state = profile.state;
	return (
		<section
			key={profile.id}
			aria-label={
				state.account?.display_name ??
				(profile.is_default
					? "Default Desktop account"
					: "Pending Desktop account")
			}
			className="space-y-3 border-t border-border pt-4"
		>
			<div>
				<p className="font-medium">
					{state.account?.display_name ??
						(profile.is_default
							? "Default Desktop session"
							: "Pending Desktop account")}
				</p>
				<p className="text-sm text-muted-foreground">
					{profile.is_default
						? "Default session"
						: state.account
							? "Isolated profile"
							: "Sign-in pending"}
				</p>
			</div>
			<div className="flex flex-wrap gap-2">
				{state.account ? (
					<>
						<Button
							type="button"
							disabled={status?.busy}
							onClick={() => void refresh(profile)}
						>
							Refresh quota
						</Button>
						<Button
							type="button"
							variant="outline"
							disabled={status?.busy || switching}
							onClick={() => void action(profile, "switch")}
						>
							{isolated
								? "Open default session"
								: `Open ${state.account.display_name}`}
						</Button>
					</>
				) : null}
				{isolated ? (
					<>
						<Button
							type="button"
							variant="outline"
							disabled={isolatedStatus?.busy || switching}
							onClick={() => void action(isolated, "switch")}
						>
							Open isolated profile
						</Button>
						<Button
							type="button"
							variant="outline"
							disabled={isolatedStatus?.busy || switching}
							onClick={() => void action(isolated, "open")}
						>
							Open isolated login
						</Button>
						<Button
							type="button"
							variant="outline"
							disabled={isolatedStatus?.busy}
							onClick={() => void action(isolated, "link")}
						>
							Confirm isolated account
						</Button>
					</>
				) : null}
				{!profile.is_default ? (
					<Button
						type="button"
						variant="outline"
						disabled={status?.busy}
						onClick={() => void action(profile, "open")}
					>
						Open login
					</Button>
				) : null}
				{!state.account || !profile.is_default ? (
					<Button
						type="button"
						variant="outline"
						disabled={status?.busy}
						onClick={() => void action(profile, "link")}
					>
						{profile.is_default ? "Link Desktop account" : "Confirm account"}
					</Button>
				) : null}
			</div>
			{isolatedStatus?.failure ? (
				<p role="alert" className="text-sm text-destructive">
					{isolatedStatus.failure}
				</p>
			) : null}
			{isolatedStatus?.notice ? (
				<p role="status" className="text-sm">
					{isolatedStatus.notice}
				</p>
			) : null}
			{status?.busy ? (
				<p role="status" className="text-sm">
					Checking Desktop account…
				</p>
			) : null}
			{status?.notice ? (
				<p role="status" className="text-sm">
					{status.notice}
				</p>
			) : null}
			{status?.failure || state.error ? (
				<p role="alert" className="text-sm text-destructive">
					{status?.failure || message(state.error)}
				</p>
			) : null}
			{state.last_attempt_at ? (
				<p className="text-sm text-muted-foreground">
					Last attempt: {time(state.last_attempt_at)}
				</p>
			) : null}
			{state.last_success ? (
				<div className="space-y-2">
					<p className="text-sm text-muted-foreground">
						{status?.current ? "Latest collection" : "Historical data"} · Last
						successful collection: {time(state.last_success.collected_at)}
					</p>
					<div className="grid gap-3 sm:grid-cols-2">
						<WindowQuota label="5 hour" window={state.last_success.five_hour} />
						<WindowQuota label="7 day" window={state.last_success.seven_day} />
					</div>
				</div>
			) : null}
		</section>
	);
}
