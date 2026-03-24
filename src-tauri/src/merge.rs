use crate::transcribe::TranscriptSegment;
use serde::{Deserialize, Serialize};

#[allow(dead_code)]
const MAX_GAP: f64 = 0.5;
#[allow(dead_code)]
const MIN_OVERLAP: f64 = 0.1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergedSegment {
    pub speaker: String,
    pub start: f64,
    pub end: f64,
    pub text: String,
}

/// Merge two speaker-labeled transcript streams into a single time-sorted list.
/// Since each stream comes from a distinct device with a known speaker, no
/// diarization is needed — segments are labeled directly.
pub fn merge_dual_transcripts(
    local: &[TranscriptSegment],
    local_speaker: &str,
    remote: &[TranscriptSegment],
    remote_speaker: &str,
) -> Vec<MergedSegment> {
    let mut result: Vec<MergedSegment> = local
        .iter()
        .map(|s| MergedSegment {
            speaker: local_speaker.to_string(),
            start: s.start,
            end: s.end,
            text: s.text.trim().to_string(),
        })
        .chain(remote.iter().map(|s| MergedSegment {
            speaker: remote_speaker.to_string(),
            start: s.start,
            end: s.end,
            text: s.text.trim().to_string(),
        }))
        .collect();

    result.sort_by(|a, b| a.start.partial_cmp(&b.start).unwrap_or(std::cmp::Ordering::Equal));

    log::info!(
        "Merged dual transcripts: {} local + {} remote = {} segments",
        local.len(),
        remote.len(),
        result.len()
    );

    result
}

/// Compute overlap duration between two time ranges
#[allow(dead_code)]
fn compute_overlap(s1_start: f64, s1_end: f64, s2_start: f64, s2_end: f64) -> f64 {
    let overlap_start = s1_start.max(s2_start);
    let overlap_end = s1_end.min(s2_end);
    (overlap_end - overlap_start).max(0.0)
}

#[allow(dead_code)]
fn find_best_speaker(
    ts: &TranscriptSegment,
    diarization: &[DiarizationSegment],
) -> Option<String> {
    let mut best_speaker = None;
    let mut best_overlap = 0.0;

    for ds in diarization {
        let overlap = compute_overlap(ts.start, ts.end, ds.start, ds.end);
        if overlap > best_overlap {
            best_overlap = overlap;
            best_speaker = Some(ds.speaker.clone());
        }
    }

    if best_overlap >= MIN_OVERLAP {
        return best_speaker;
    }

    // Proximity fallback
    find_nearest_speaker(ts, diarization)
}

#[allow(dead_code)]
fn find_nearest_speaker(
    ts: &TranscriptSegment,
    diarization: &[DiarizationSegment],
) -> Option<String> {
    let seg_mid = (ts.start + ts.end) / 2.0;
    let mut nearest = None;
    let mut min_dist = f64::INFINITY;

    for ds in diarization {
        let speaker_mid = (ds.start + ds.end) / 2.0;
        let dist = (seg_mid - speaker_mid).abs();
        if dist < min_dist {
            min_dist = dist;
            nearest = Some(ds.speaker.clone());
        }
    }

    if min_dist <= MAX_GAP {
        nearest
    } else {
        None
    }
}

use crate::diarize::DiarizationSegment;

/// Merge transcript segments with diarization segments (legacy — used for single-stream dropped audio).
#[allow(dead_code)]
pub fn merge(
    transcript: &[TranscriptSegment],
    diarization: &[DiarizationSegment],
) -> Vec<MergedSegment> {
    if transcript.is_empty() || diarization.is_empty() {
        return transcript
            .iter()
            .map(|ts| MergedSegment {
                speaker: "UNKNOWN".to_string(),
                start: ts.start,
                end: ts.end,
                text: ts.text.trim().to_string(),
            })
            .collect();
    }

    log::info!(
        "Merging {} transcript segments with {} diarization segments",
        transcript.len(),
        diarization.len()
    );

    let mut assigned = Vec::new();
    let mut unassigned = Vec::new();

    for ts in transcript {
        let merged = MergedSegment {
            speaker: String::new(),
            start: ts.start,
            end: ts.end,
            text: ts.text.trim().to_string(),
        };

        if let Some(speaker) = find_best_speaker(ts, diarization) {
            let mut m = merged;
            m.speaker = speaker;
            assigned.push(m);
        } else {
            unassigned.push(merged);
        }
    }

    // Assign remaining using context
    assign_remaining(&mut assigned, &mut unassigned);

    let mut all = assigned;
    all.extend(unassigned);
    all.sort_by(|a, b| a.start.partial_cmp(&b.start).unwrap());

    let total_speakers: std::collections::HashSet<&str> =
        all.iter().map(|s| s.speaker.as_str()).collect();
    log::info!(
        "Merged into {} segments with {} speakers",
        all.len(),
        total_speakers.len()
    );

    all
}

#[allow(dead_code)]
fn assign_remaining(assigned: &mut Vec<MergedSegment>, unassigned: &mut Vec<MergedSegment>) {
    if assigned.is_empty() {
        for seg in unassigned.iter_mut() {
            seg.speaker = "UNKNOWN".to_string();
        }
        return;
    }

    assigned.sort_by(|a, b| a.start.partial_cmp(&b.start).unwrap());

    let mut newly_assigned = Vec::new();

    for (i, seg) in unassigned.iter_mut().enumerate() {
        // Find previous assigned segment
        let prev = assigned.iter().rev().find(|m| m.end <= seg.start);
        // Find next assigned segment
        let next = assigned.iter().find(|m| m.start >= seg.end);

        match (prev, next) {
            (Some(p), Some(n)) => {
                let gap_prev = seg.start - p.end;
                let gap_next = n.start - seg.end;
                if gap_prev <= gap_next && gap_prev <= MAX_GAP {
                    seg.speaker = p.speaker.clone();
                    newly_assigned.push(i);
                } else if gap_next <= MAX_GAP {
                    seg.speaker = n.speaker.clone();
                    newly_assigned.push(i);
                } else {
                    seg.speaker = "UNKNOWN".to_string();
                }
            }
            (Some(p), None) => {
                if seg.start - p.end <= MAX_GAP {
                    seg.speaker = p.speaker.clone();
                    newly_assigned.push(i);
                } else {
                    seg.speaker = "UNKNOWN".to_string();
                }
            }
            (None, Some(n)) => {
                if n.start - seg.end <= MAX_GAP {
                    seg.speaker = n.speaker.clone();
                    newly_assigned.push(i);
                } else {
                    seg.speaker = "UNKNOWN".to_string();
                }
            }
            (None, None) => {
                seg.speaker = "UNKNOWN".to_string();
            }
        }
    }
}
