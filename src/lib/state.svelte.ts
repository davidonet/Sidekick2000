import type { Speaker, AppPhase, PipelineStep, ContextFile, CreatedIssue } from "./types";

export const CONTEXT_FILES: ContextFile[] = [
  { label: "Default (General)", filename: "default.md" },
  { label: "Welqin (IP Platform)", filename: "welqin.md" },
  { label: "Affinity (Serif Training)", filename: "affinity.md" },
];

export const DEFAULT_SPEAKERS: Speaker[] = [
  { name: "David", organization: "Welqin", enabled: true },
  { name: "Yannick", organization: "Welqin", enabled: true },
  { name: "Marc", organization: "Welqin", enabled: true },
  { name: "Adrien", organization: "Welqin", enabled: true },
];

export const LANGUAGES = [
  { code: "fr", label: "French" },
  { code: "en", label: "English" },
  { code: "es", label: "Spanish" },
  { code: "de", label: "German" },
  { code: "it", label: "Italian" },
  { code: "pt", label: "Portuguese" },
  { code: "", label: "Auto-detect" },
];

class AppState {
  phase: AppPhase = $state("setup");

  // Setup
  contextFile: string = $state("default.md");
  customContext: string = $state("");
  language: string = $state("fr");
  outputDir: string = $state("");
  speakers: Speaker[] = $state(structuredClone(DEFAULT_SPEAKERS));

  // Recording
  audioLevel: number = $state(0);
  elapsedSecs: number = $state(0);

  // Processing
  pipelineStep: PipelineStep = $state("transcribing");
  pipelineProgress: number = $state(0);

  // Result
  resultPath: string = $state("");
  errorMessage: string = $state("");

  // GitHub
  githubRepo: string = $state("");
  createdIssues: CreatedIssue[] = $state([]);

  // Recording file paths (set after stop)
  oggPath: string = $state("");
  wavPath: string = $state("");

  get enabledSpeakers(): Speaker[] {
    return this.speakers.filter((s) => s.enabled);
  }

  get formattedTime(): string {
    const mins = Math.floor(this.elapsedSecs / 60);
    const secs = Math.floor(this.elapsedSecs % 60);
    return `${String(mins).padStart(2, "0")}:${String(secs).padStart(2, "0")}`;
  }

  reset() {
    this.phase = "setup";
    this.audioLevel = 0;
    this.elapsedSecs = 0;
    this.pipelineStep = "transcribing";
    this.pipelineProgress = 0;
    this.resultPath = "";
    this.errorMessage = "";
    this.createdIssues = [];
    this.oggPath = "";
    this.wavPath = "";
  }
}

export const appState = new AppState();
