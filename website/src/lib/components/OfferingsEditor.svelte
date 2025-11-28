<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import { importProviderOfferingsCSV, type CsvImportResult, hexEncode, downloadOfferingsCSV, getExampleOfferingsCSV, getProductTypes, type ProductType } from '$lib/services/api';
	import { signRequest } from '$lib/services/auth-api';
	import type { Ed25519KeyIdentity } from '@dfinity/identity';
	import SpreadsheetEditor from './SpreadsheetEditor.svelte';
	import { onMount } from 'svelte';

	interface Props {
		open?: boolean;
		identity: Ed25519KeyIdentity | null;
		pubkeyBytes: Uint8Array | null;
		csvContent?: string;
	}

	let { open = $bindable(false), identity, pubkeyBytes, csvContent = '' }: Props = $props();

	const dispatch = createEventDispatcher<{
		success: CsvImportResult;
		close: void;
	}>();

	let isDragging = $state(false);
	let selectedFile = $state<File | null>(null);
	let currentCsvContent = $state('');
	let fullCsvContent = $state(''); // Store the complete CSV with all product types
	let importing = $state(false);
	let error = $state<string | null>(null);
	let result = $state<CsvImportResult | null>(null);
	let selectedResourceType = $state<string>('');
	let productTypes = $state<ProductType[]>([]);
	let loadingExample = $state(false);

	// Resource type definitions with key columns (keeping for reference, but will fetch from backend)
	const RESOURCE_TYPES = {
		compute: {
			label: '💻 Compute / VPS',
			columns: ['processor_brand', 'processor_cores', 'processor_speed', 'memory_amount', 'total_ssd_capacity']
		},
		gpu: {
			label: '🎮 GPU / AI',
			columns: ['gpu_name', 'gpu_count', 'gpu_memory_gb', 'processor_cores', 'memory_amount']
		},
		storage: {
			label: '💾 Storage',
			columns: ['total_hdd_capacity', 'total_ssd_capacity', 'uplink_speed', 'traffic']
		},
		network: {
			label: '🌐 Network / CDN',
			columns: ['uplink_speed', 'traffic', 'unmetered_bandwidth', 'datacenter_country']
		},
		dedicated: {
			label: '🖥️ Dedicated Server',
			columns: ['processor_brand', 'processor_name', 'processor_cores', 'memory_amount', 'total_hdd_capacity', 'total_ssd_capacity']
		}
	} as const;

	type ResourceTypeKey = keyof typeof RESOURCE_TYPES;

	// Fetch product types on mount
	onMount(async () => {
		try {
			productTypes = await getProductTypes();
		} catch (e) {
			console.error('Failed to fetch product types:', e);
			// Fallback to hardcoded types
			productTypes = [
				{ key: 'compute', label: '💻 Compute / VPS' },
				{ key: 'gpu', label: '🎮 GPU / AI' },
				{ key: 'storage', label: '💾 Storage' },
				{ key: 'network', label: '🌐 Network / CDN' },
				{ key: 'dedicated', label: '🖥️ Dedicated Server' }
			];
		}
	});

	// Load CSV content when dialog opens
	$effect(() => {
		if (open && csvContent) {
			fullCsvContent = csvContent; // Store the full CSV
			currentCsvContent = csvContent;
			selectedFile = null;
			error = null;
			result = null;
		}
	});

	// Parse CSV and filter by product type
	function filterCsvByProductType(csv: string, productType: string): string {
		if (!csv || !productType) return csv;

		const lines = csv.split('\n');
		if (lines.length === 0) return '';

		const header = lines[0];
		const headers = header.split(',').map(h => h.trim().replace(/^"|"$/g, ''));
		const productTypeIndex = headers.indexOf('product_type');

		if (productTypeIndex === -1) {
			// No product_type column, return as is
			return csv;
		}

		// Filter data rows by product_type
		const dataLines = lines.slice(1);
		const filteredLines = dataLines.filter(line => {
			if (!line.trim()) return false;

			// Simple CSV parsing (handles quoted values)
			const values: string[] = [];
			let current = '';
			let inQuotes = false;

			for (let i = 0; i < line.length; i++) {
				const char = line[i];
				if (char === '"') {
					inQuotes = !inQuotes;
				} else if (char === ',' && !inQuotes) {
					values.push(current.trim());
					current = '';
				} else {
					current += char;
				}
			}
			values.push(current.trim());

			const rowProductType = values[productTypeIndex]?.replace(/^"|"$/g, '').trim();
			return rowProductType === productType;
		});

		return [header, ...filteredLines].join('\n');
	}

	// Check if spreadsheet is empty (only headers or completely empty)
	function isSpreadsheetEmpty(): boolean {
		const content = currentCsvContent.trim();
		if (!content) return true;

		const lines = content.split('\n').filter(line => line.trim());
		// Empty if no lines or only one line (header)
		return lines.length <= 1;
	}

	// Load example data for the selected product type (internal, no warning)
	async function loadExampleDataInternal() {
		if (!selectedResourceType) {
			error = 'Please select a product type first';
			return;
		}

		loadingExample = true;
		error = null;

		try {
			const exampleCSV = await getExampleOfferingsCSV(selectedResourceType);
			currentCsvContent = exampleCSV;
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to load example data';
			console.error('Error loading example data:', e);
		} finally {
			loadingExample = false;
		}
	}

	// Load example data with confirmation if data exists
	async function loadExampleData() {
		// Check if there's existing data
		if (!isSpreadsheetEmpty()) {
			const confirmed = confirm(
				'Warning: This will replace your existing data with example data. This action cannot be undone. Do you want to continue?'
			);
			if (!confirmed) {
				return;
			}
		}

		await loadExampleDataInternal();
	}

	// Handle product type button click - filter user's offerings by product type
	function handleProductTypeClick(productType: string) {
		selectedResourceType = productType;
		// Filter the user's existing offerings by the selected product type
		currentCsvContent = filterCsvByProductType(fullCsvContent, productType);
	}

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
			const fileContent = await file.text();
			fullCsvContent = fileContent;

			// If a product type is selected, filter the loaded content
			if (selectedResourceType) {
				currentCsvContent = filterCsvByProductType(fileContent, selectedResourceType);
			} else {
				currentCsvContent = fileContent;
			}
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to read file';
			selectedFile = null;
		}
	}

	async function handleSave() {
		if (!identity || !pubkeyBytes || !currentCsvContent) {
			error = 'Missing authentication or CSV content';
			return;
		}

		importing = true;
		error = null;
		result = null;

		try {
			const pubkeyHex = hexEncode(pubkeyBytes);
			// Always use upsert mode for editing
			const path = `/api/v1/providers/${pubkeyHex}/offerings/import?upsert=true`;

			// Sign the request - this returns the exact body that was signed
			const signed = await signRequest(identity, 'POST', path, currentCsvContent, 'text/plain');

			if (!signed.body) {
				throw new Error('Failed to sign request: signed body is empty');
			}

			// DEBUG: Log what we're signing and sending
			console.log('=== SIGNATURE DEBUG ===');
			console.log('Original CSV length:', currentCsvContent.length);
			console.log('Signed body length:', signed.body.length);
			console.log('Bodies match:', currentCsvContent === signed.body);
			console.log('Timestamp:', signed.headers['X-Timestamp']);
			console.log('Path:', path);
			console.log('First 200 chars of CSV:', currentCsvContent.substring(0, 200));
			console.log('First 200 chars of signed:', signed.body.substring(0, 200));
			console.log('=======================');

			// CRITICAL: Use signed.body (the exact string that was signed) not currentCsvContent
			const importResult = await importProviderOfferingsCSV(
				pubkeyBytes,
				signed.body,
				true, // Always upsert
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
			error = e instanceof Error ? e.message : 'Save failed';
			console.error('Save error:', e);
		} finally {
			importing = false;
		}
	}

	function handleClose() {
		open = false;
		selectedFile = null;
		currentCsvContent = '';
		error = null;
		result = null;
		isDragging = false;
		selectedResourceType = '';
		dispatch('close');
	}

	function handleDownload() {
		if (!currentCsvContent) return;
		const blob = new Blob([currentCsvContent], { type: 'text/csv;charset=utf-8;' });
		const link = document.createElement('a');
		const url = URL.createObjectURL(blob);
		
		link.setAttribute('href', url);
		link.setAttribute('download', 'offerings.csv');
		link.style.visibility = 'hidden';
		document.body.appendChild(link);
		link.click();
		document.body.removeChild(link);
		URL.revokeObjectURL(url);
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
			class="bg-gradient-to-br from-slate-900 to-slate-800 rounded-2xl shadow-2xl border border-white/20 w-full max-w-6xl max-h-[90vh] overflow-y-auto"
		>
			<!-- Header -->
			<div class="flex items-center justify-between p-6 border-b border-white/10">
				<div>
					<h2 class="text-2xl font-bold text-white">Edit Offerings</h2>
					<p class="text-white/60 text-sm mt-1">
						Edit your offerings directly or upload a CSV file
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
				<!-- Action Buttons -->
				<div class="flex flex-wrap gap-3 items-center justify-between">
					<div class="flex gap-3">
						<label
							class="px-4 py-2 bg-white/10 rounded-lg font-medium hover:bg-white/20 transition-all cursor-pointer flex items-center gap-2"
						>
							<svg class="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
								<path
									stroke-linecap="round"
									stroke-linejoin="round"
									stroke-width="2"
									d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12"
								/>
							</svg>
							Upload CSV
							<input
								type="file"
								accept=".csv"
								onchange={handleFileSelect}
								class="hidden"
								disabled={importing}
								aria-label="Upload CSV file"
							/>
						</label>
						<button
							onclick={handleDownload}
							class="px-4 py-2 bg-white/10 rounded-lg font-medium hover:bg-white/20 transition-all flex items-center gap-2"
							disabled={importing || !currentCsvContent}
						>
							<svg class="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
								<path
									stroke-linecap="round"
									stroke-linejoin="round"
									stroke-width="2"
									d="M12 10v6m0 0l-3-3m3 3l3-3m2 8H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"
								/>
							</svg>
							Download CSV
						</button>
					</div>
					{#if selectedFile}
						<div class="flex items-center gap-2 text-sm text-white/70">
							<span class="text-lg">📄</span>
							<span>{selectedFile.name}</span>
						</div>
					{/if}
				</div>

				<!-- Product Type Selector -->
				<div class="space-y-4">
					<div>
						<div class="text-white/70 text-sm font-medium block mb-3">Product Type:</div>
						<div class="flex flex-wrap gap-2">
							{#each productTypes as { key, label }}
								<button
									onclick={() => handleProductTypeClick(key)}
									class="px-4 py-3 rounded-lg font-medium transition-all {selectedResourceType === key
										? 'bg-blue-600 text-white'
										: 'bg-white/10 text-white/70 hover:bg-white/20'}"
									disabled={importing || loadingExample}
								>
									{label}
								</button>
							{/each}
						</div>
					</div>

					<!-- Load Example Data Button -->
					{#if selectedResourceType}
						<div class="flex items-center gap-3">
							<button
								onclick={loadExampleData}
								class="px-4 py-2 rounded-lg font-medium transition-all flex items-center gap-2 {isSpreadsheetEmpty()
									? 'bg-gradient-to-r from-green-500 to-emerald-600 text-white animate-pulse hover:scale-105'
									: 'bg-white/10 text-white/70 hover:bg-white/20'}"
								disabled={importing || loadingExample}
							>
								{#if loadingExample}
									<svg class="animate-spin h-4 w-4" viewBox="0 0 24 24">
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
									Loading...
								{:else}
									<svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
										<path
											stroke-linecap="round"
											stroke-linejoin="round"
											stroke-width="2"
											d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4"
										/>
									</svg>
									Load Example Data
								{/if}
							</button>
							{#if isSpreadsheetEmpty()}
								<span class="text-emerald-400 text-sm">
									← Click to load example data for {productTypes.find(t => t.key === selectedResourceType)?.label}
								</span>
							{/if}
						</div>
					{/if}
				</div>

				<!-- CSV Spreadsheet Editor -->
				<div class="space-y-3">
					<SpreadsheetEditor
						bind:value={currentCsvContent}
						disabled={importing}
						onchange={(csv) => (currentCsvContent = csv)}
					/>
				</div>

				<!-- Drag and Drop Overlay -->
				{#if isDragging}
					<!-- svelte-ignore a11y_no_static_element_interactions -->
					<div
						class="absolute inset-0 bg-blue-500/20 border-4 border-dashed border-blue-500 rounded-2xl flex items-center justify-center z-10"
						ondragenter={handleDragEnter}
						ondragleave={handleDragLeave}
						ondragover={handleDragOver}
						ondrop={handleDrop}
					>
						<div class="text-center">
							<div class="text-6xl mb-4">📄</div>
							<p class="text-white text-2xl font-bold">Drop CSV file here</p>
						</div>
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
								Successfully saved {result.success_count} offering{result.success_count !== 1
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

				<!-- Instructions -->
				<div class="bg-blue-500/10 border border-blue-500/30 rounded-lg p-4 space-y-2">
					<p class="text-blue-400 font-semibold mb-2">📋 How to Edit Offerings</p>
					<ul class="text-white/70 text-sm space-y-1 list-disc list-inside">
						<li>Edit offerings directly in the spreadsheet above</li>
						<li>Click "Download CSV" to export your offerings for editing in Excel or Google Sheets</li>
						<li>Click "Upload CSV" to import offerings from a file</li>
						<li>Drag and drop a CSV file anywhere to upload</li>
						<li>Changes will update existing offerings and create new ones as needed</li>
					</ul>
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
					onclick={handleSave}
					class="px-6 py-3 bg-gradient-to-r from-blue-500 to-purple-600 rounded-lg font-semibold hover:brightness-110 hover:scale-105 transition-all disabled:opacity-50 disabled:cursor-not-allowed disabled:hover:scale-100"
					disabled={!currentCsvContent || importing || !!result}
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
							Saving...
						</span>
					{:else}
						Save Changes
					{/if}
				</button>
			</div>
		</div>
	</div>
	<!-- Drag overlay for entire window -->
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div
		class="fixed inset-0 z-40 pointer-events-none"
		ondragenter={handleDragEnter}
		ondragleave={handleDragLeave}
		ondragover={handleDragOver}
		ondrop={handleDrop}
	></div>
{/if}
