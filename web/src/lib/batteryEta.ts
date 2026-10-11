import type { BatteryInfo } from '../api';

export function formatEtaMinutes(totalMinutes: number): string {
	if (!isFinite(totalMinutes) || totalMinutes <= 0) return '—';
	const m = Math.round(totalMinutes);
	const days = Math.floor(m / 1440);
	const hours = Math.floor((m % 1440) / 60);
	const mins = m % 60;
	if (days > 0) {
		// cap to two units
		return hours > 0 ? `${days}d ${hours}h` : `${days}d`;
	}
	if (hours > 0) {
		return mins > 0 ? `${hours}h ${mins}m` : `${hours}h`;
	}
	return `${mins}m`;
}

export function minutesToChargeTarget(info: BatteryInfo | undefined, targetPct: number): number | undefined {
	if (!info?.present_rate_ma || info.present_rate_ma <= 0) return undefined;
	if (!info.charging) return undefined;
	const lfcc = info.last_full_charge_capacity_mah;
	const currentPct = info.percentage;
	const remainingMah = info.remaining_capacity_mah;
	if (lfcc != null && currentPct != null) {
		const deltaPct = Math.max(0, targetPct - currentPct);
		const deltaMah = (lfcc * deltaPct) / 100;
		if (deltaMah <= 0) return 0;
		return (deltaMah / info.present_rate_ma) * 60;
	}
	if (lfcc != null && remainingMah != null) {
		const targetMah = Math.max(0, Math.floor((lfcc * targetPct) / 100));
		const deltaMah = Math.max(0, targetMah - remainingMah);
		if (deltaMah <= 0) return 0;
		return (deltaMah / info.present_rate_ma) * 60;
	}
	return undefined;
}

/** Minutes until a discharging battery that sits above `targetPct` drains down to it. */
export function minutesToDrainTarget(info: BatteryInfo | undefined, targetPct: number): number | undefined {
	if (!info?.present_rate_ma || info.present_rate_ma <= 0) return undefined;
	if (info.charging) return undefined;
	const lfcc = info.last_full_charge_capacity_mah;
	const currentPct = info.percentage;
	const remainingMah = info.remaining_capacity_mah;
	if (lfcc != null && currentPct != null && currentPct > targetPct) {
		const deltaPct = currentPct - targetPct;
		const deltaMah = (lfcc * deltaPct) / 100;
		if (deltaMah <= 0) return 0;
		return (deltaMah / info.present_rate_ma) * 60;
	}
	if (lfcc != null && remainingMah != null && Math.round((remainingMah * 100) / lfcc) > targetPct) {
		const targetMah = Math.max(0, Math.floor((lfcc * targetPct) / 100));
		const deltaMah = Math.max(0, remainingMah - targetMah);
		if (deltaMah <= 0) return 0;
		return (deltaMah / info.present_rate_ma) * 60;
	}
	return undefined;
}

export function minutesToEmpty(info: BatteryInfo | undefined): number | undefined {
	if (!info?.present_rate_ma || info.present_rate_ma <= 0) return undefined;
	if (info.charging) return undefined;
	const remainingMah = info.remaining_capacity_mah;
	if (remainingMah == null) return undefined;
	return (remainingMah / info.present_rate_ma) * 60;
}
