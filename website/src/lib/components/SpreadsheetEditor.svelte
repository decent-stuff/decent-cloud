<script lang="ts">
	interface Props {
		value?: string;
		disabled?: boolean;
		onchange?: (csv: string) => void;
	}

	let { value = $bindable(''), disabled = false, onchange }: Props = $props();

	let rows = $state<string[][]>([]);
	let selectedCell = $state<{ row: number; col: number } | null>(null);
	let lastExternalValue = '';
	let updateTimeout: ReturnType<typeof setTimeout> | null = null;

	// Parse CSV to 2D array (handles quoted values)
	function parseCSV(csv: string): string[][] {
		if (!csv.trim()) {
			return [['']];
		}
		const lines = csv.split('\n').filter((line) => line.trim());
		return lines.map((line) => {
			const cells: string[] = [];
			let current = '';
			let inQuotes = false;

			for (let i = 0; i < line.length; i++) {
				const char = line[i];
				if (char === '"') {
					inQuotes = !inQuotes;
				} else if (char === ',' && !inQuotes) {
					cells.push(current.trim());
					current = '';
				} else {
					current += char;
				}
			}
			cells.push(current.trim());
			return cells;
		});
	}

	// Convert 2D array to CSV (RFC 4180 compliant)
	function toCSV(data: string[][]): string {
		return data.map((row) => 
			row.map((cell) => {
				// Quote field if it contains comma, quote, or newline
				if (cell.includes(',') || cell.includes('"') || cell.includes('\n')) {
					// Escape quotes by doubling them
					return '"' + cell.replace(/"/g, '""') + '"';
				}
				return cell;
			}).join(',')
		).join('\n');
	}

	// Initialize rows from value (only when value changes externally)
	$effect(() => {
		if (value && value !== lastExternalValue) {
			lastExternalValue = value;
			rows = parseCSV(value);
		}
	});

	// Update value when rows change (flush immediately)
	function updateValue() {
		if (updateTimeout) {
			clearTimeout(updateTimeout);
			updateTimeout = null;
		}
		const csv = toCSV(rows);
		value = csv;
		lastExternalValue = csv;
		onchange?.(csv);
	}

	// Debounced update for typing performance
	function scheduleUpdate() {
		if (updateTimeout) clearTimeout(updateTimeout);
		updateTimeout = setTimeout(updateValue, 150);
	}

	function updateCell(rowIndex: number, colIndex: number, newValue: string) {
		rows[rowIndex][colIndex] = newValue;
		scheduleUpdate();
	}

	function addRow() {
		const colCount = rows[0]?.length || 1;
		rows = [...rows, Array(colCount).fill('')];
		updateValue();
	}

	function deleteRow(rowIndex: number) {
		// Don't delete header row (index 0) or if only header + 1 data row remains
		if (rowIndex > 0 && rows.length > 2) {
			rows = rows.filter((_, i) => i !== rowIndex);
			updateValue();
		}
	}

	function handleCellClick(rowIndex: number, colIndex: number) {
		if (!disabled) {
			selectedCell = { row: rowIndex, col: colIndex };
		}
	}

	function handleCellBlur() {
		// Flush any pending debounced update
		if (updateTimeout) updateValue();
		selectedCell = null;
	}

	function handleKeyDown(e: KeyboardEvent, rowIndex: number, colIndex: number) {
		if (disabled) return;

		switch (e.key) {
			case 'Tab':
				e.preventDefault();
				const nextCol = e.shiftKey ? colIndex - 1 : colIndex + 1;
				if (nextCol >= 0 && nextCol < rows[rowIndex].length) {
					selectedCell = { row: rowIndex, col: nextCol };
				}
				break;
			case 'Enter':
				e.preventDefault();
				const nextRow = rowIndex + 1;
				if (nextRow < rows.length) {
					selectedCell = { row: nextRow, col: colIndex };
				}
				break;
		}
	}
</script>

<div class="bg-glass/5 backdrop-blur-lg rounded-xl border border-glass/15 overflow-hidden">
	<!-- Header Controls -->
	<div class="flex items-center justify-between p-4 border-b border-glass/10">
		<div class="text-white/70 text-sm font-medium">
			{rows.length - 1} data rows × {rows[0]?.length || 0} columns
		</div>
		<button
			onclick={addRow}
			{disabled}
			class="px-4 py-2 bg-glass/10 rounded-lg text-sm text-white/90 hover:bg-glass/15 transition-all disabled:opacity-50 disabled:cursor-not-allowed flex items-center gap-2"
			title="Add new data row"
		>
			<svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
				<path
					stroke-linecap="round"
					stroke-linejoin="round"
					stroke-width="2"
					d="M12 4v16m8-8H4"
				/>
			</svg>
			Add Row
		</button>
	</div>

	<!-- Spreadsheet -->
	<div class="overflow-x-auto max-h-[60vh]">
		<table class="w-full border-collapse">
			<!-- Headers (first row, non-editable) -->
			<thead>
				<tr class="bg-glass/10 border-b-2 border-glass/15">
					<th class="px-2 py-2 text-white/40 text-xs sticky left-0 bg-glass/10">#</th>
					{#each rows[0] || [] as header, colIndex}
						<th class="px-3 py-2 min-w-32 text-left">
							<span class="text-primary-400 font-semibold text-sm truncate block" title={header}>
								{header || `Column ${colIndex + 1}`}
							</span>
						</th>
					{/each}
				</tr>
			</thead>
			<tbody>
				<!-- Data rows (skip first row as it's the header) -->
				{#each rows.slice(1) as row, rowIndex}
					<tr class="border-b border-white/5 hover:bg-glass/5 transition-colors group">
						<!-- Row number + delete button -->
						<td class="px-2 py-1 text-white/40 text-xs sticky left-0 bg-glass/5 border-r border-glass/10">
							<div class="flex items-center gap-1">
								<span class="w-6 text-center">{rowIndex + 1}</span>
								{#if rows.length > 2 && !disabled}
									<button
										onclick={() => deleteRow(rowIndex + 1)}
										class="opacity-0 group-hover:opacity-100 transition-opacity text-red-400 hover:text-red-300"
										title="Delete row"
									>
										<svg class="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor">
											<path
												stroke-linecap="round"
												stroke-linejoin="round"
												stroke-width="2"
												d="M6 18L18 6M6 6l12 12"
											/>
										</svg>
									</button>
								{/if}
							</div>
						</td>

						<!-- Cells -->
						{#each row as cell, colIndex}
							<td class="px-1 py-1 min-w-32 relative">
								{#if selectedCell?.row === rowIndex + 1 && selectedCell?.col === colIndex}
									<input
										type="text"
										value={cell}
										oninput={(e) =>
											updateCell(rowIndex + 1, colIndex, (e.target as HTMLInputElement).value)}
										onblur={handleCellBlur}
										onkeydown={(e) => handleKeyDown(e, rowIndex + 1, colIndex)}
										class="w-full px-2 py-1.5 bg-glass/10 border border-primary-500 rounded text-white text-sm focus:outline-none focus:ring-2 focus:ring-primary-400"
									/>
								{:else}
									<!-- svelte-ignore a11y_click_events_have_key_events -->
									<!-- svelte-ignore a11y_no_static_element_interactions -->
									<div
										onclick={() => handleCellClick(rowIndex + 1, colIndex)}
										class="px-2 py-1.5 text-white/90 text-sm rounded cursor-pointer hover:bg-glass/10 transition-colors truncate"
										title={cell}
									>
										{cell || '\u00A0'}
									</div>
								{/if}
							</td>
						{/each}
					</tr>
				{/each}
			</tbody>
		</table>
	</div>

	<!-- Footer -->
	<div class="p-3 border-t border-glass/10 bg-glass/5">
		<p class="text-white/50 text-xs">
			💡 Click a cell to edit • Tab to move right • Enter to move down • Headers are shown at top and are non-editable
		</p>
	</div>
</div>
