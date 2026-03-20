import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { PipelineConfig, PipelineResult, PipelineStep } from "./types";

export async function startRecording(): Promise<void> {
  await invoke("start_recording");
}

export async function stopRecording(): Promise<[string, string]> {
  return await invoke("stop_recording");
}

export async function getAudioLevel(): Promise<number> {
  return await invoke("get_audio_level");
}

export async function getElapsed(): Promise<number> {
  return await invoke("get_elapsed");
}

export async function isRecording(): Promise<boolean> {
  return await invoke("is_recording");
}

export async function runPipeline(config: PipelineConfig): Promise<PipelineResult> {
  return await invoke("run_pipeline", { config });
}

export async function getDefaultOutputDir(): Promise<string> {
  return await invoke("get_default_output_dir");
}

export async function openFile(path: string): Promise<void> {
  await invoke("open_file", { path });
}

export async function loadContextFile(name: string): Promise<string> {
  return await invoke("load_context_file", { name });
}

export function onPipelineProgress(
  callback: (step: PipelineStep, progress: number) => void,
) {
  return listen<{ step: PipelineStep; progress: number }>(
    "pipeline-progress",
    (event) => {
      callback(event.payload.step, event.payload.progress);
    },
  );
}

export function onAudioLevel(
  callback: (level: number, elapsed: number) => void,
) {
  return listen<{ level: number; elapsed_secs: number }>(
    "audio-level",
    (event) => {
      callback(event.payload.level, event.payload.elapsed_secs);
    },
  );
}
