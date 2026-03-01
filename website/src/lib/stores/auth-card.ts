import { writable } from 'svelte/store';

/** True when an AuthRequiredCard is mounted on the current page. */
export const authCardVisible = writable(false);
