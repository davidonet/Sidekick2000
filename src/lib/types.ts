export interface Speaker {
  name: string;
  organization: string;
  enabled: boolean;
}

export interface PipelineConfig {
  context: string;
  context_content: string;
  speakers: { name: string; organization: string }[];
  language_code: string;
  language_name: string;
  github_repo: string;
  output_dir: string;
  ogg_path: string;
  wav_path: string;
}

export interface CreatedIssue {
  number: number;
  title: string;
  url: string;
}

export interface PipelineResult {
  notes_path: string;
  created_issues: CreatedIssue[];
}

export type AppPhase = "setup" | "recording" | "processing" | "result" | "error";

export type PipelineStep =
  | "transcribing"
  | "diarizing"
  | "merging"
  | "summarizing"
  | "exporting"
  | "done";

export interface ContextFile {
  label: string;
  filename: string;
}
