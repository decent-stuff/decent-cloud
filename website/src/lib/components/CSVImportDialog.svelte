<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import { importProviderOfferingsCSV, type CsvImportResult, hexEncode } from '$lib/services/api';
	import { signRequest } from '$lib/services/auth-api';
	import type { Ed25519KeyIdentity } from '@dfinity/identity';
	import SpreadsheetEditor from './SpreadsheetEditor.svelte';

	interface Props {
		open?: boolean;
		identity: Ed25519KeyIdentity | null;
		pubkeyBytes: Uint8Array | null;
		prefilledContent?: string;
	}

	let { open = $bindable(false), identity, pubkeyBytes, prefilledContent = '' }: Props = $props();

	const dispatch = createEventDispatcher<{
		success: CsvImportResult;
		close: void;
	}>();

	let isDragging = $state(false);
	let selectedFile = $state<File | null>(null);
	let csvContent = $state('');
	let editMode = $state(false);
	let upsertMode = $state(false);
	let importing = $state(false);
	let error = $state<string | null>(null);
	let result = $state<CsvImportResult | null>(null);

	// Load prefilled content when dialog opens
	$effect(() => {
		if (open && prefilledContent) {
			csvContent = prefilledContent;
			editMode = true;
			selectedFile = null;
			error = null;
			result = null;
		}
	});

	function handleDragEnter(e: DragEvent) {
		e.preventDefault();
		isDragging = true;
	}

	function handleDragLeave(e: DragEvent) {
		e.preventDefault();
		isDragging = false;
	}

	function handleDragOver(e: DragEvent) {
		e.preventDefault();
	}

	async function handleDrop(e: DragEvent) {
		e.preventDefault();
		isDragging = false;

		const files = e.dataTransfer?.files;
		if (files && files.length > 0) {
			await loadFile(files[0]);
		}
	}

	async function handleFileSelect(e: Event) {
		const input = e.target as HTMLInputElement;
		if (input.files && input.files.length > 0) {
			await loadFile(input.files[0]);
		}
	}

	async function loadFile(file: File) {
		if (!file.name.toLowerCase().endsWith('.csv')) {
			error = 'Please select a CSV file';
			return;
		}

		selectedFile = file;
		error = null;
		result = null;

		try {
			csvContent = await file.text();
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to read file';
			selectedFile = null;
		}
	}

	async function handleImport() {
		if (!identity || !pubkeyBytes || !csvContent) {
			error = 'Missing authentication or CSV content';
			return;
		}

		importing = true;
		error = null;
		result = null;

		try {
			const pubkeyHex = hexEncode(pubkeyBytes);
			const path = `/api/v1/providers/${pubkeyHex}/offerings/import${upsertMode ? '?upsert=true' : ''}`;

			// Sign the request - for CSV we need to pass the CSV content as body
			const signed = await signRequest(identity, 'POST', path, undefined);

			// Import CSV with signed headers but CSV body
			const importResult = await importProviderOfferingsCSV(
				pubkeyBytes,
				csvContent,
				upsertMode,
				signed.headers
			);

			result = importResult;

			if (importResult.errors.length === 0) {
				setTimeout(() => {
					dispatch('success', importResult);
					handleClose();
				}, 2000);
			}
		} catch (e) {
			error = e instanceof Error ? e.message : 'Import failed';
			console.error('Import error:', e);
		} finally {
			importing = false;
		}
	}

	function handleClose() {
		open = false;
		selectedFile = null;
		csvContent = '';
		editMode = false;
		error = null;
		result = null;
		isDragging = false;
		dispatch('close');
	}

	function resetFile() {
		selectedFile = null;
		csvContent = '';
		editMode = false;
		error = null;
		result = null;
	}

	function enableEditMode() {
		editMode = true;
		selectedFile = null;
	}
</script>

{#if open}
	<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<div
		class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-sm p-4"
		onclick={(e) => e.target === e.currentTarget && handleClose()}
		role="dialog"
		aria-modal="true"
		tabindex="-1"
	>
		<div
			class="bg-gradient-to-br from-slate-900 to-slate-800 rounded-2xl shadow-2xl border border-white/20 w-full max-w-2xl max-h-[90vh] overflow-y-auto"
		>
			<!-- Header -->
			<div class="flex items-center justify-between p-6 border-b border-white/10">
				<div>
					<h2 class="text-2xl font-bold text-white">Import / Create Offerings</h2>
					<p class="text-white/60 text-sm mt-1">
						Upload a CSV file to bulk import or create offerings
					</p>
				</div>
				<button
					onclick={handleClose}
					class="text-white/60 hover:text-white transition-colors"
					aria-label="Close dialog"
				>
					<svg class="w-6 h-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
						<path
							stroke-linecap="round"
							stroke-linejoin="round"
							stroke-width="2"
							d="M6 18L18 6M6 6l12 12"
						/>
					</svg>
				</button>
			</div>

			<!-- Content -->
			<div class="p-6 space-y-6">
				<!-- Upsert Mode Toggle -->
				<label class="flex items-center gap-3 cursor-pointer group">
					<input type="checkbox" bind:checked={upsertMode} class="sr-only peer" />
					<div
						class="relative w-11 h-6 bg-white/20 peer-focus:outline-none peer-focus:ring-2 peer-focus:ring-blue-500 rounded-full peer peer-checked:after:translate-x-full peer-checked:bg-blue-600 after:content-[''] after:absolute after:top-0.5 after:left-[2px] after:bg-white after:rounded-full after:h-5 after:w-5 after:transition-all"
					></div>
					<div>
						<div class="text-white font-medium group-hover:text-blue-400 transition-colors">
							Update Existing Offerings
						</div>
						<div class="text-white/60 text-sm">
							When enabled, existing offerings with matching IDs will be updated
						</div>
					</div>
				</label>

				<!-- CSV Editor or File Upload -->
				{#if editMode}
					<!-- CSV Spreadsheet Editor -->
					<div class="space-y-3">
						<div class="flex items-center justify-between">
							<div class="text-white font-medium">
								Edit Offerings Spreadsheet
							</div>
							<button
								onclick={resetFile}
								class="text-white/60 hover:text-white text-sm transition-colors"
								disabled={importing}
							>
								Switch to File Upload
							</button>
						</div>
						<SpreadsheetEditor
							bind:value={csvContent}
							disabled={importing}
							onchange={(csv) => (csvContent = csv)}
						/>
					</div>
				{:else if !selectedFile}
					<!-- File Upload Area -->
					<!-- svelte-ignore a11y_no_static_element_interactions -->
					<div
						class="border-2 border-dashed rounded-xl p-12 text-center transition-all {isDragging
							? 'border-blue-500 bg-blue-500/10'
							: 'border-white/20 bg-white/5 hover:border-white/40 hover:bg-white/10'}"
						ondragenter={handleDragEnter}
						ondragleave={handleDragLeave}
						ondragover={handleDragOver}
						ondrop={handleDrop}
						role="button"
						tabindex="0"
					>
						<div class="text-6xl mb-4">📄</div>
						<p class="text-white text-lg font-medium mb-2">
							{isDragging ? 'Drop CSV file here' : 'Drag and drop CSV file'}
						</p>
						<p class="text-white/60 text-sm mb-4">or</p>
						<div class="flex gap-3 justify-center">
							<label
								class="inline-block px-6 py-3 bg-gradient-to-r from-blue-500 to-purple-600 rounded-lg font-semibold cursor-pointer hover:brightness-110 hover:scale-105 transition-all"
							>
								Browse Files
								<input
									type="file"
									accept=".csv"
									onchange={handleFileSelect}
									class="hidden"
									aria-label="Select CSV file"
								/>
							</label>
							<button
								onclick={enableEditMode}
								class="px-6 py-3 bg-white/10 rounded-lg font-semibold hover:bg-white/20 transition-all"
							>
								Edit CSV Directly
							</button>
						</div>
					</div>
				{:else}
					<!-- Selected File Info -->
					<div
						class="bg-white/10 border border-white/20 rounded-xl p-4 flex items-center justify-between"
					>
						<div class="flex items-center gap-3">
							<span class="text-3xl">📄</span>
							<div>
								<p class="text-white font-medium">{selectedFile.name}</p>
								<p class="text-white/60 text-sm">
									{(selectedFile.size / 1024).toFixed(2)} KB
								</p>
							</div>
						</div>
						<button
							onclick={resetFile}
							class="text-red-400 hover:text-red-300 transition-colors"
							disabled={importing}
							aria-label="Remove file"
						>
							<svg class="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
								<path
									stroke-linecap="round"
									stroke-linejoin="round"
									stroke-width="2"
									d="M6 18L18 6M6 6l12 12"
								/>
							</svg>
						</button>
					</div>
				{/if}

				<!-- Error Display -->
				{#if error}
					<div class="bg-red-500/20 border border-red-500/30 rounded-lg p-4">
						<p class="text-red-400 font-semibold">Error</p>
						<p class="text-red-400/80 text-sm mt-1">{error}</p>
					</div>
				{/if}

				<!-- Result Display -->
				{#if result}
					<div
						class="bg-green-500/20 border border-green-500/30 rounded-lg p-4 space-y-3"
					>
						<div class="flex items-center gap-2">
							<span class="text-2xl">✅</span>
							<p class="text-green-400 font-semibold">
								Successfully imported {result.success_count} offering{result.success_count !== 1
									? 's'
									: ''}
							</p>
						</div>

						{#if result.errors.length > 0}
							<div class="mt-4 pt-4 border-t border-green-500/30">
								<p class="text-yellow-400 font-semibold mb-2">
									⚠️ {result.errors.length} row{result.errors.length !== 1 ? 's' : ''} had errors:
								</p>
								<div class="max-h-48 overflow-y-auto space-y-2">
									{#each result.errors as err}
										<div class="bg-black/30 rounded p-2 text-sm">
											<span class="text-yellow-400 font-medium">Row {err.row}:</span>
											<span class="text-white/80 ml-2">{err.message}</span>
										</div>
									{/each}
								</div>
							</div>
						{/if}
					</div>
				{/if}

				<!-- Format Information -->
				<div class="bg-blue-500/10 border border-blue-500/30 rounded-lg p-4 space-y-3">
					<div>
						<p class="text-blue-400 font-semibold mb-2">📋 How to Create Offerings</p>
						<ol class="text-white/70 text-sm space-y-2 list-decimal list-inside">
							<li>Click "Download Template" button to get a CSV with examples</li>
							<li>Open the CSV in Excel, Google Sheets, or any spreadsheet app</li>
							<li>Edit the example rows or add new ones with your offerings</li>
							<li>Save the file and upload it here</li>
							<li>
								Toggle "Update Existing Offerings" if you want to update offerings with matching
								IDs
							</li>
						</ol>
					</div>
					<div class="pt-3 border-t border-blue-500/20">
						<p class="text-white/60 text-xs">
							<strong>Tip:</strong> The template includes 2 example offerings (VM and Dedicated Server)
							that you can use as reference.
						</p>
					</div>
				</div>
			</div>

			<!-- Footer Actions -->
			<div class="flex items-center justify-end gap-3 p-6 border-t border-white/10">
				<button
					onclick={handleClose}
					class="px-6 py-3 bg-white/10 rounded-lg font-medium hover:bg-white/20 transition-all"
					disabled={importing}
				>
					Cancel
				</button>
				<button
					onclick={handleImport}
					class="px-6 py-3 bg-gradient-to-r from-blue-500 to-purple-600 rounded-lg font-semibold hover:brightness-110 hover:scale-105 transition-all disabled:opacity-50 disabled:cursor-not-allowed disabled:hover:scale-100"
					disabled={(!selectedFile && !editMode) || !csvContent || importing || !!result}
				>
					{#if importing}
						<span class="flex items-center gap-2">
							<svg class="animate-spin h-5 w-5" viewBox="0 0 24 24">
								<circle
									class="opacity-25"
									cx="12"
									cy="12"
									r="10"
									stroke="currentColor"
									stroke-width="4"
									fill="none"
								/>
								<path
									class="opacity-75"
									fill="currentColor"
									d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
								/>
							</svg>
							Importing...
						</span>
					{:else}
						Import Offerings
					{/if}
				</button>
			</div>
		</div>
	</div>
{/if}
