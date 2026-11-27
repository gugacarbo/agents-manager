import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useCallback, useEffect, useRef, useState } from "react";
import {
	createClaudeDesktopQuotaProfile,
	type DesktopQuotaProfile_Serialize,
	linkClaudeDesktopQuotaAccount,
	listClaudeDesktopQuotaProfiles,
	openClaudeDesktopQuotaLogin,
	openClaudeDesktopQuotaProfile,
	refreshClaudeDesktopQuota,
} from "@/lib/claude-accounts";
import {
	actionFailureMessage,
	type Display,
	message,
	refreshNotice,
} from "./desktop-quota-display";

type Profile = DesktopQuotaProfile_Serialize;

const claudeDesktopQuotaProfilesQueryKey = [
	"claude-desktop-quota-profiles",
] as const;

export function useClaudeDesktopQuotas() {
	const queryClient = useQueryClient();
	const [display, setDisplay] = useState<Record<string, Display>>({});
	const [switching, setSwitching] = useState(false);
	const locks = useRef(new Set<string>());
	const switchLock = useRef(false);
	const addLock = useRef(false);
	const mounted = useRef(true);
	const entryRefreshed = useRef(false);

	useEffect(() => {
		mounted.current = true;
		return () => {
			mounted.current = false;
		};
	}, []);

	const update = useCallback((id: string, patch: Display) => {
		if (mounted.current) {
			setDisplay((previous) => ({
				...previous,
				[id]: { ...previous[id], ...patch },
			}));
		}
	}, []);

	const replace = useCallback(
		(profile: Profile, fallbackItems?: Profile[]) => {
			queryClient.setQueryData<Profile[]>(
				claudeDesktopQuotaProfilesQueryKey,
				(previous) => {
					const list = previous ?? fallbackItems;
					if (!list) return [profile];
					return list.map((item) => (item.id === profile.id ? profile : item));
				},
			);
		},
		[queryClient],
	);

	const { mutateAsync: refreshQuota } = useMutation({
		mutationFn: (profileId: string) => refreshClaudeDesktopQuota(profileId),
	});
	const { mutateAsync: linkQuotaAccount } = useMutation({
		mutationFn: (profileId: string) => linkClaudeDesktopQuotaAccount(profileId),
	});
	const { mutateAsync: openDesktopSession } = useMutation({
		mutationFn: ({ id, kind }: { id: string; kind: "open" | "switch" }) =>
			kind === "switch"
				? openClaudeDesktopQuotaProfile(id)
				: openClaudeDesktopQuotaLogin(id),
	});
	const addMutation = useMutation({
		mutationFn: () => createClaudeDesktopQuotaProfile(),
	});

	const refresh = useCallback(
		async (profile: Profile, fallbackItems?: Profile[]) => {
			if (locks.current.has(profile.id)) return;
			locks.current.add(profile.id);
			update(profile.id, {
				busy: true,
				failure: "",
				notice: "",
				current: false,
			});
			try {
				const result = await refreshQuota(profile.id);
				replace({ ...profile, state: result.state }, fallbackItems);
				update(profile.id, {
					current: result.status === "updated",
					notice: refreshNotice(result),
				});
			} catch (error) {
				update(profile.id, { failure: message(error) });
			} finally {
				locks.current.delete(profile.id);
				update(profile.id, { busy: false });
			}
		},
		[refreshQuota, replace, update],
	);

	const profilesQuery = useQuery({
		queryKey: claudeDesktopQuotaProfilesQueryKey,
		queryFn: async () => {
			const result = await listClaudeDesktopQuotaProfiles();
			if (!entryRefreshed.current) {
				entryRefreshed.current = true;
				for (const profile of result) {
					if (profile.state.account && !profile.shares_default_quota)
						void refresh(profile, result);
				}
			}
			return result;
		},
		refetchOnWindowFocus: false,
		refetchOnReconnect: false,
	});

	async function action(profile: Profile, kind: "link" | "open" | "switch") {
		if (
			locks.current.has(profile.id) ||
			(kind === "switch" && switchLock.current)
		)
			return;
		if (kind === "switch") {
			switchLock.current = true;
			setSwitching(true);
		}
		locks.current.add(profile.id);
		update(profile.id, {
			busy: true,
			failure: "",
			notice: "",
			...(kind === "switch" ? {} : { current: false }),
		});
		try {
			if (kind === "link") {
				replace(await linkQuotaAccount(profile.id));
				if (
					profile.is_default &&
					profiles.some((item) => !item.is_default && item.state.account)
				) {
					await queryClient.invalidateQueries({
						queryKey: claudeDesktopQuotaProfilesQueryKey,
					});
				}
			} else {
				await openDesktopSession({ id: profile.id, kind });
				if (kind === "open") {
					update(profile.id, {
						notice:
							"Complete sign-in, close Claude Desktop, then confirm the account.",
					});
				}
			}
		} catch (error) {
			update(profile.id, {
				failure: actionFailureMessage(error, kind, profile.is_default),
			});
		} finally {
			locks.current.delete(profile.id);
			if (kind === "switch") {
				switchLock.current = false;
				setSwitching(false);
			}
			update(profile.id, { busy: false });
		}
	}

	async function add() {
		if (addLock.current || addMutation.isPending) return;
		addLock.current = true;
		addMutation.reset();
		try {
			const profile = await addMutation.mutateAsync();
			if (!mounted.current) return;
			queryClient.setQueryData<Profile[]>(
				claudeDesktopQuotaProfilesQueryKey,
				(previous) => [...(previous ?? []), profile],
			);
			await action(profile, "open");
		} catch {
			// Displayed via addMutation.error.
		} finally {
			addLock.current = false;
		}
	}

	const profiles = profilesQuery.data ?? [];
	const loading = profilesQuery.isPending;
	const failure = profilesQuery.isError
		? message(profilesQuery.error)
		: addMutation.isError
			? message(addMutation.error)
			: "";

	return {
		profiles,
		display,
		loading,
		adding: addMutation.isPending,
		switching,
		failure,
		refresh,
		action,
		add,
	};
}
