import { writable } from 'svelte/store';
import { DefaultService } from '../api';

/** Starts true so laptops never flash their battery UI away; false once the service reports no battery. */
export const batteryPresent = writable(true);

let checked = false;

export function checkBatteryPresence(): void {
	if (checked) return;
	checked = true;
	DefaultService.getPower()
		.then((resp) => batteryPresent.set(resp.battery?.battery_present !== false))
		.catch((e) => {
			checked = false;
			console.warn('[battery] presence check failed', e);
		});
}
