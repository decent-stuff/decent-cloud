<script lang="ts" module>
	// Icon names for type safety
	export type IconName =
		| 'shield'
		| 'star'
		| 'clock'
		| 'alert'
		| 'wallet'
		| 'eye'
		| 'server'
		| 'users'
		| 'code'
		| 'cpu'
		| 'target'
		| 'zap'
		| 'check'
		| 'arrow-right'
		| 'arrow-down'
		| 'menu'
		| 'x'
		| 'cart'
		| 'package'
		| 'file'
		| 'settings'
		| 'logout'
		| 'login'
		| 'user'
		| 'headphones'
		| 'wrench'
		| 'inbox'
		| 'bot'
		| 'briefcase'
		| 'external'
		| 'chart'
		| 'lock'
		| 'unlock'
		| 'refresh'
		| 'trending-up'
		| 'search'
		| 'filter'
		| 'grid'
		| 'list'
		| 'chevron-down'
		| 'chevron-right'
		| 'plus'
		| 'minus'
		| 'copy'
		| 'download'
		| 'upload'
		| 'link'
		| 'globe'
		| 'mail'
		| 'phone'
		| 'building'
		| 'database'
		| 'cloud'
		| 'terminal'
		| 'activity'
		| 'gpu'
		| 'chevron-up'
		| 'repeat'
		| 'hard-drive'
		| 'sparkles'
		| 'key'
		| 'arrow-left';
</script>

<script lang="ts">
	interface Props {
		name: IconName;
		size?: number;
		class?: string;
	}

	let { name, size = 20, class: className = '' }: Props = $props();
</script>

<svg
	width={size}
	height={size}
	viewBox="0 0 24 24"
	fill="none"
	stroke="currentColor"
	stroke-width="2"
	stroke-linecap="round"
	stroke-linejoin="round"
	class={className}
	aria-hidden="true"
>
	{#if name === 'shield'}
		<path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z" />
	{:else if name === 'star'}
		<polygon points="12 2 15.09 8.26 22 9.27 17 14.14 18.18 21.02 12 17.77 5.82 21.02 7 14.14 2 9.27 8.91 8.26 12 2" />
	{:else if name === 'clock'}
		<circle cx="12" cy="12" r="10" />
		<polyline points="12 6 12 12 16 14" />
	{:else if name === 'alert'}
		<path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z" />
		<line x1="12" y1="9" x2="12" y2="13" />
		<line x1="12" y1="17" x2="12.01" y2="17" />
	{:else if name === 'wallet'}
		<path d="M21 4H3a2 2 0 0 0-2 2v12a2 2 0 0 0 2 2h18a2 2 0 0 0 2-2V6a2 2 0 0 0-2-2z" />
		<path d="M16 12h5" />
		<circle cx="18" cy="12" r="1" fill="currentColor" stroke="none" />
	{:else if name === 'eye'}
		<path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z" />
		<circle cx="12" cy="12" r="3" />
	{:else if name === 'server'}
		<rect x="2" y="2" width="20" height="8" rx="2" ry="2" />
		<rect x="2" y="14" width="20" height="8" rx="2" ry="2" />
		<line x1="6" y1="6" x2="6.01" y2="6" />
		<line x1="6" y1="18" x2="6.01" y2="18" />
	{:else if name === 'users'}
		<path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2" />
		<circle cx="9" cy="7" r="4" />
		<path d="M23 21v-2a4 4 0 0 0-3-3.87" />
		<path d="M16 3.13a4 4 0 0 1 0 7.75" />
	{:else if name === 'code'}
		<polyline points="16 18 22 12 16 6" />
		<polyline points="8 6 2 12 8 18" />
	{:else if name === 'cpu'}
		<rect x="4" y="4" width="16" height="16" rx="2" ry="2" />
		<rect x="9" y="9" width="6" height="6" />
		<line x1="9" y1="1" x2="9" y2="4" />
		<line x1="15" y1="1" x2="15" y2="4" />
		<line x1="9" y1="20" x2="9" y2="23" />
		<line x1="15" y1="20" x2="15" y2="23" />
		<line x1="20" y1="9" x2="23" y2="9" />
		<line x1="20" y1="14" x2="23" y2="14" />
		<line x1="1" y1="9" x2="4" y2="9" />
		<line x1="1" y1="14" x2="4" y2="14" />
	{:else if name === 'target'}
		<circle cx="12" cy="12" r="10" />
		<circle cx="12" cy="12" r="6" />
		<circle cx="12" cy="12" r="2" />
	{:else if name === 'zap'}
		<polygon points="13 2 3 14 12 14 11 22 21 10 12 10 13 2" />
	{:else if name === 'check'}
		<polyline points="20 6 9 17 4 12" />
	{:else if name === 'arrow-right'}
		<line x1="5" y1="12" x2="19" y2="12" />
		<polyline points="12 5 19 12 12 19" />
	{:else if name === 'arrow-down'}
		<line x1="12" y1="5" x2="12" y2="19" />
		<polyline points="19 12 12 19 5 12" />
	{:else if name === 'menu'}
		<line x1="3" y1="6" x2="21" y2="6" />
		<line x1="3" y1="12" x2="21" y2="12" />
		<line x1="3" y1="18" x2="21" y2="18" />
	{:else if name === 'x'}
		<line x1="18" y1="6" x2="6" y2="18" />
		<line x1="6" y1="6" x2="18" y2="18" />
	{:else if name === 'cart'}
		<circle cx="9" cy="21" r="1" />
		<circle cx="20" cy="21" r="1" />
		<path d="M1 1h4l2.68 13.39a2 2 0 0 0 2 1.61h9.72a2 2 0 0 0 2-1.61L23 6H6" />
	{:else if name === 'package'}
		<line x1="16.5" y1="9.4" x2="7.5" y2="4.21" />
		<path d="M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z" />
		<polyline points="3.27 6.96 12 12.01 20.73 6.96" />
		<line x1="12" y1="22.08" x2="12" y2="12" />
	{:else if name === 'file'}
		<path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
		<polyline points="14 2 14 8 20 8" />
	{:else if name === 'settings'}
		<circle cx="12" cy="12" r="3" />
		<path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z" />
	{:else if name === 'logout'}
		<path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4" />
		<polyline points="16 17 21 12 16 7" />
		<line x1="21" y1="12" x2="9" y2="12" />
	{:else if name === 'login'}
		<path d="M15 3h4a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2h-4" />
		<polyline points="10 17 15 12 10 7" />
		<line x1="15" y1="12" x2="3" y2="12" />
	{:else if name === 'user'}
		<path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2" />
		<circle cx="12" cy="7" r="4" />
	{:else if name === 'headphones'}
		<path d="M3 18v-6a9 9 0 0 1 18 0v6" />
		<path d="M21 19a2 2 0 0 1-2 2h-1a2 2 0 0 1-2-2v-3a2 2 0 0 1 2-2h3zM3 19a2 2 0 0 0 2 2h1a2 2 0 0 0 2-2v-3a2 2 0 0 0-2-2H3z" />
	{:else if name === 'wrench'}
		<path d="M14.7 6.3a1 1 0 0 0 0 1.4l1.6 1.6a1 1 0 0 0 1.4 0l3.77-3.77a6 6 0 0 1-7.94 7.94l-6.91 6.91a2.12 2.12 0 0 1-3-3l6.91-6.91a6 6 0 0 1 7.94-7.94l-3.76 3.76z" />
	{:else if name === 'inbox'}
		<polyline points="22 12 16 12 14 15 10 15 8 12 2 12" />
		<path d="M5.45 5.11L2 12v6a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2v-6l-3.45-6.89A2 2 0 0 0 16.76 4H7.24a2 2 0 0 0-1.79 1.11z" />
	{:else if name === 'bot'}
		<rect x="3" y="11" width="18" height="10" rx="2" />
		<circle cx="12" cy="5" r="2" />
		<path d="M12 7v4" />
		<line x1="8" y1="16" x2="8" y2="16" />
		<line x1="16" y1="16" x2="16" y2="16" />
	{:else if name === 'briefcase'}
		<rect x="2" y="7" width="20" height="14" rx="2" ry="2" />
		<path d="M16 21V5a2 2 0 0 0-2-2h-4a2 2 0 0 0-2 2v16" />
	{:else if name === 'external'}
		<path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6" />
		<polyline points="15 3 21 3 21 9" />
		<line x1="10" y1="14" x2="21" y2="3" />
	{:else if name === 'chart'}
		<line x1="18" y1="20" x2="18" y2="10" />
		<line x1="12" y1="20" x2="12" y2="4" />
		<line x1="6" y1="20" x2="6" y2="14" />
	{:else if name === 'lock'}
		<rect x="3" y="11" width="18" height="11" rx="2" ry="2" />
		<path d="M7 11V7a5 5 0 0 1 10 0v4" />
	{:else if name === 'unlock'}
		<rect x="3" y="11" width="18" height="11" rx="2" ry="2" />
		<path d="M7 11V7a5 5 0 0 1 9.9-1" />
	{:else if name === 'refresh'}
		<polyline points="23 4 23 10 17 10" />
		<polyline points="1 20 1 14 7 14" />
		<path d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15" />
	{:else if name === 'trending-up'}
		<polyline points="23 6 13.5 15.5 8.5 10.5 1 18" />
		<polyline points="17 6 23 6 23 12" />
	{:else if name === 'search'}
		<circle cx="11" cy="11" r="8" />
		<line x1="21" y1="21" x2="16.65" y2="16.65" />
	{:else if name === 'filter'}
		<polygon points="22 3 2 3 10 12.46 10 19 14 21 14 12.46 22 3" />
	{:else if name === 'grid'}
		<rect x="3" y="3" width="7" height="7" />
		<rect x="14" y="3" width="7" height="7" />
		<rect x="14" y="14" width="7" height="7" />
		<rect x="3" y="14" width="7" height="7" />
	{:else if name === 'list'}
		<line x1="8" y1="6" x2="21" y2="6" />
		<line x1="8" y1="12" x2="21" y2="12" />
		<line x1="8" y1="18" x2="21" y2="18" />
		<line x1="3" y1="6" x2="3.01" y2="6" />
		<line x1="3" y1="12" x2="3.01" y2="12" />
		<line x1="3" y1="18" x2="3.01" y2="18" />
	{:else if name === 'chevron-down'}
		<polyline points="6 9 12 15 18 9" />
	{:else if name === 'chevron-right'}
		<polyline points="9 18 15 12 9 6" />
	{:else if name === 'plus'}
		<line x1="12" y1="5" x2="12" y2="19" />
		<line x1="5" y1="12" x2="19" y2="12" />
	{:else if name === 'minus'}
		<line x1="5" y1="12" x2="19" y2="12" />
	{:else if name === 'copy'}
		<rect x="9" y="9" width="13" height="13" rx="2" ry="2" />
		<path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" />
	{:else if name === 'download'}
		<path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
		<polyline points="7 10 12 15 17 10" />
		<line x1="12" y1="15" x2="12" y2="3" />
	{:else if name === 'upload'}
		<path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
		<polyline points="17 8 12 3 7 8" />
		<line x1="12" y1="3" x2="12" y2="15" />
	{:else if name === 'link'}
		<path d="M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71" />
		<path d="M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71" />
	{:else if name === 'globe'}
		<circle cx="12" cy="12" r="10" />
		<line x1="2" y1="12" x2="22" y2="12" />
		<path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z" />
	{:else if name === 'mail'}
		<path d="M4 4h16c1.1 0 2 .9 2 2v12c0 1.1-.9 2-2 2H4c-1.1 0-2-.9-2-2V6c0-1.1.9-2 2-2z" />
		<polyline points="22,6 12,13 2,6" />
	{:else if name === 'phone'}
		<path d="M22 16.92v3a2 2 0 0 1-2.18 2 19.79 19.79 0 0 1-8.63-3.07 19.5 19.5 0 0 1-6-6 19.79 19.79 0 0 1-3.07-8.67A2 2 0 0 1 4.11 2h3a2 2 0 0 1 2 1.72 12.84 12.84 0 0 0 .7 2.81 2 2 0 0 1-.45 2.11L8.09 9.91a16 16 0 0 0 6 6l1.27-1.27a2 2 0 0 1 2.11-.45 12.84 12.84 0 0 0 2.81.7A2 2 0 0 1 22 16.92z" />
	{:else if name === 'building'}
		<rect x="4" y="2" width="16" height="20" rx="2" ry="2" />
		<path d="M9 22v-4h6v4" />
		<line x1="8" y1="6" x2="8" y2="6" />
		<line x1="16" y1="6" x2="16" y2="6" />
		<line x1="12" y1="6" x2="12" y2="6" />
		<line x1="8" y1="10" x2="8" y2="10" />
		<line x1="16" y1="10" x2="16" y2="10" />
		<line x1="12" y1="10" x2="12" y2="10" />
		<line x1="8" y1="14" x2="8" y2="14" />
		<line x1="16" y1="14" x2="16" y2="14" />
		<line x1="12" y1="14" x2="12" y2="14" />
	{:else if name === 'database'}
		<ellipse cx="12" cy="5" rx="9" ry="3" />
		<path d="M21 12c0 1.66-4 3-9 3s-9-1.34-9-3" />
		<path d="M3 5v14c0 1.66 4 3 9 3s9-1.34 9-3V5" />
	{:else if name === 'cloud'}
		<path d="M18 10h-1.26A8 8 0 1 0 9 20h9a5 5 0 0 0 0-10z" />
	{:else if name === 'terminal'}
		<polyline points="4 17 10 11 4 5" />
		<line x1="12" y1="19" x2="20" y2="19" />
	{:else if name === 'activity'}
		<polyline points="22 12 18 12 15 21 9 3 6 12 2 12" />
	{:else if name === 'gpu'}
		<rect x="2" y="6" width="20" height="12" rx="2" />
		<line x1="6" y1="10" x2="6" y2="14" />
		<line x1="10" y1="10" x2="10" y2="14" />
		<line x1="14" y1="10" x2="14" y2="14" />
		<line x1="18" y1="10" x2="18" y2="14" />
	{:else if name === 'chevron-up'}
		<polyline points="18 15 12 9 6 15" />
	{:else if name === 'repeat'}
		<polyline points="17 1 21 5 17 9" />
		<path d="M3 11V9a4 4 0 0 1 4-4h14" />
		<polyline points="7 23 3 19 7 15" />
		<path d="M21 13v2a4 4 0 0 1-4 4H3" />
	{:else if name === 'hard-drive'}
		<line x1="22" y1="12" x2="2" y2="12" />
		<path d="M5.45 5.11L2 12v6a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2v-6l-3.45-6.89A2 2 0 0 0 16.76 4H7.24a2 2 0 0 0-1.79 1.11z" />
		<line x1="6" y1="16" x2="6.01" y2="16" />
		<line x1="10" y1="16" x2="10.01" y2="16" />
	{:else if name === 'sparkles'}
		<path d="M12 3l1.5 4.5L18 9l-4.5 1.5L12 15l-1.5-4.5L6 9l4.5-1.5L12 3z" />
		<path d="M5 19l.5 1.5L7 21l-1.5.5L5 23l-.5-1.5L3 21l1.5-.5L5 19z" />
		<path d="M19 13l.5 1.5L21 15l-1.5.5L19 17l-.5-1.5L17 15l1.5-.5L19 13z" />
	{:else if name === 'key'}
		<path d="M21 2l-2 2m-7.61 7.61a5.5 5.5 0 1 1-7.78 7.78 5.5 5.5 0 0 1 7.78-7.78zm0 0L15.5 7.5m0 0l3 3L22 7l-3-3m-3.5 3.5L19 4" />
	{:else if name === 'arrow-left'}
		<line x1="19" y1="12" x2="5" y2="12" />
		<polyline points="12 19 5 12 12 5" />
	{/if}
</svg>
