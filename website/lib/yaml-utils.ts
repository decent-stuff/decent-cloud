import yaml from 'js-yaml';

/**
 * Detects if a string is likely YAML format
 * @param str The string to check
 * @returns true if the string appears to be YAML, false otherwise
 */
export function isYamlFormat(str: string): boolean {
    // Simple heuristic - YAML often starts with --- or has key: value pattern
    // without having JSON structure like {} or []
    const trimmed = str.trim();

    // If it starts with typical JSON patterns, it's likely not YAML
    if (trimmed.startsWith('{') && trimmed.endsWith('}')) {
        try {
            JSON.parse(trimmed);
            return false; // If it's valid JSON, it's not YAML
        } catch {
            // If JSON.parse fails, it might be YAML with { } characters
        }
    }

    // Check for common YAML patterns
    const hasYamlIndentation = /^[\w#].*:\s*\S/m.test(trimmed);
    const hasDocumentStart = trimmed.startsWith('---');
    const hasMappingStructure = /^[\w#][^:]*:\s*\S/m.test(trimmed);

    return hasYamlIndentation || hasDocumentStart || hasMappingStructure;
}

/**
 * Converts a YAML string to a JSON string
 * @param yamlStr The YAML string to convert
 * @returns JSON string representation
 * @throws Error if parsing fails
 */
export function yamlToJson(yamlStr: string): string {
    try {
        const parsed = yaml.load(yamlStr);
        return JSON.stringify(parsed, null, 0);
    } catch (error) {
        throw new Error(`Failed to parse YAML: ${error instanceof Error ? error.message : String(error)}`);
    }
}

/**
 * Validates if a string is valid YAML
 * @param yamlStr The YAML string to validate
 * @returns true if valid YAML, false otherwise
 */
export function validateYaml(yamlStr: string): boolean {
    try {
        yaml.load(yamlStr);
        return true;
    } catch {
        return false;
    }
}

/**
 * Validates a YAML string and returns detailed error information
 * @param yamlStr The YAML string to validate
 * @returns An object with success flag and optional error details
 */
export function validateYamlDetailed(yamlStr: string): {
    valid: boolean;
    error?: {
        message: string;
        line?: number;
        column?: number;
    };
} {
    try {
        yaml.load(yamlStr);
        return { valid: true };
    } catch (e) {
        if (e instanceof Error) {
            // Parse error message to extract line and column if available
            const errorMessage = e.message;
            const lineMatch = errorMessage.match(/line (\d+)/);
            const columnMatch = errorMessage.match(/column (\d+)/);

            return {
                valid: false,
                error: {
                    message: errorMessage,
                    line: lineMatch ? parseInt(lineMatch[1]) : undefined,
                    column: columnMatch ? parseInt(columnMatch[1]) : undefined
                }
            };
        }
        return {
            valid: false,
            error: { message: String(e) }
        };
    }
}

/**
 * Validates a JSON string and returns detailed error information
 * @param jsonStr The JSON string to validate
 * @returns An object with success flag and optional error details
 */
export function validateJsonDetailed(jsonStr: string): {
    valid: boolean;
    error?: {
        message: string;
        position?: number;
    };
} {
    try {
        JSON.parse(jsonStr);
        return { valid: true };
    } catch (e) {
        if (e instanceof Error) {
            const errorMessage = e.message;
            const positionMatch = errorMessage.match(/at position (\d+)/);

            return {
                valid: false,
                error: {
                    message: errorMessage,
                    position: positionMatch ? parseInt(positionMatch[1]) : undefined
                }
            };
        }
        return {
            valid: false,
            error: { message: String(e) }
        };
    }
}
