"use client";

import { useEffect, useRef, useState } from "react";
import * as monaco from "monaco-editor";
import { validateYamlDetailed, validateJsonDetailed } from "@/lib/yaml-utils";
import { Editor } from "@monaco-editor/react";
import { offeringJsonSchema } from "@/lib/offering-schema";

interface CodeEditorProps {
  value: string;
  onChange: (value: string) => void;
  language: "json" | "yaml";
  height?: string;
}

export default function CodeEditor({
  value,
  onChange,
  language,
  height = "400px",
}: CodeEditorProps) {
  const [errors, setErrors] = useState<monaco.editor.IMarkerData[]>([]);
  const editorRef = useRef<monaco.editor.IStandaloneCodeEditor | null>(null);
  const monacoRef = useRef<typeof monaco | null>(null);

  // Set up Monaco Editor
  const handleEditorDidMount = (
    editor: monaco.editor.IStandaloneCodeEditor,
    monaco: typeof import("monaco-editor")
  ) => {
    editorRef.current = editor;
    monacoRef.current = monaco;

    // Set theme to dark
    monaco.editor.defineTheme("decentCloud", {
      base: "vs-dark",
      inherit: true,
      rules: [
        // YAML specific syntax highlighting
        { token: "keyword.yaml", foreground: "#569CD6" },
        { token: "string.yaml", foreground: "#CE9178" },
        { token: "number.yaml", foreground: "#B5CEA8" },
        { token: "comment.yaml", foreground: "#6A9955" },

        // JSON specific syntax highlighting
        { token: "string.key.json", foreground: "#9CDCFE" },
        { token: "string.value.json", foreground: "#CE9178" },
        { token: "number.json", foreground: "#B5CEA8" },
        { token: "keyword.json", foreground: "#569CD6" },
      ],
      colors: {
        "editor.background": "#13151a",
        "editor.foreground": "#d1d5db",
        "editor.lineHighlightBackground": "#2e3440",
        "editorLineNumber.foreground": "#6b7280",
        "editorCursor.foreground": "#d1d5db",
      },
    });
    monaco.editor.setTheme("decentCloud");

    // Register JSON schema for validation
    monaco.languages.json.jsonDefaults.setDiagnosticsOptions({
      validate: true,
      schemas: [
        {
          uri: "http://decent-cloud/schemas/offering.json",
          fileMatch: ["*"],
          schema: offeringJsonSchema,
        },
      ],
    });
  };

  // Handle editor value change
  const handleEditorChange = (value: string | undefined) => {
    if (value !== undefined) {
      onChange(value);
      validateContent(value, language);
    }
  };

  // Validate content based on language
  const validateContent = (content: string, lang: "json" | "yaml") => {
    if (!monacoRef.current || !editorRef.current) return;

    const monaco = monacoRef.current;
    const editor = editorRef.current;
    const model = editor.getModel();
    if (!model) return;

    let markers: monaco.editor.IMarkerData[] = [];

    if (lang === "json") {
      const result = validateJsonDetailed(content);

      if (!result.valid && result.error) {
        const position = result.error.position || 0;
        const pos = model.getPositionAt(position);

        markers = [
          {
            severity: monaco.MarkerSeverity.Error,
            message: result.error.message,
            startLineNumber: pos.lineNumber,
            startColumn: pos.column,
            endLineNumber: pos.lineNumber,
            endColumn: pos.column + 1,
          },
        ];
      }
    } else if (lang === "yaml") {
      const result = validateYamlDetailed(content);

      if (!result.valid && result.error) {
        // If we have line and column information
        if (result.error.line && result.error.column) {
          const line = result.error.line;
          const column = result.error.column;

          markers = [
            {
              severity: monaco.MarkerSeverity.Error,
              message: result.error.message,
              startLineNumber: line,
              startColumn: column,
              endLineNumber: line,
              endColumn: column + 1,
            },
          ];
        } else {
          // Without specific location info, mark the entire document
          markers = [
            {
              severity: monaco.MarkerSeverity.Error,
              message: result.error.message,
              startLineNumber: 1,
              startColumn: 1,
              endLineNumber: model.getLineCount(),
              endColumn: model.getLineMaxColumn(model.getLineCount()),
            },
          ];
        }
      }
    }

    monaco.editor.setModelMarkers(model, "validation", markers);
    setErrors(markers);
  };

  // Update validation when language changes
  useEffect(() => {
    if (value) {
      validateContent(value, language);
    }
  }, [language, value]);

  return (
    <div className="code-editor-container">
      <Editor
        height={height}
        language={language}
        value={value}
        onChange={handleEditorChange}
        onMount={handleEditorDidMount}
        options={{
          minimap: { enabled: false },
          scrollBeyondLastLine: false,
          fontSize: 14,
          lineNumbers: "on",
          wordWrap: "on",
          automaticLayout: true,
          tabSize: 2,
          folding: true,
          foldingStrategy: "indentation",
          formatOnPaste: true,
          formatOnType: true,
          renderLineHighlight: "all",
          suggestOnTriggerCharacters: true,
          autoIndent: "full",
          contextmenu: true,
        }}
      />
      {errors.length > 0 && (
        <div className="bg-red-900/50 text-red-200 p-2 mt-1 rounded text-sm">
          {errors.map((error, index) => (
            <div key={index}>
              <span className="font-bold">Line {error.startLineNumber}:</span>{" "}
              {error.message}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
