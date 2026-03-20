use anyhow::{Context, Result};
use ndarray::{Array2, Axis};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Parameters for diarization
const FRAME_LENGTH: f64 = 0.025; // 25ms
const FRAME_SHIFT: f64 = 0.010; // 10ms
const N_MFCC: usize = 20;
const MIN_SEGMENT_DUR: f64 = 0.5;
const VAD_THRESHOLD: f32 = 0.3;
const VAD_PAD_DUR: f64 = 0.1;
const N_MELS: usize = 40;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiarizationSegment {
    pub speaker: String,
    pub start: f64,
    pub end: f64,
}

/// Speech segment detected by VAD
#[derive(Debug, Clone)]
struct SpeechSegment {
    start: f64,
    end: f64,
}

/// Run speaker diarization on a WAV file
pub fn diarize(
    wav_path: &Path,
    min_speakers: usize,
    max_speakers: usize,
) -> Result<Vec<DiarizationSegment>> {
    log::info!("Starting diarization of {}", wav_path.display());

    // 1. Load WAV
    let mut reader =
        hound::WavReader::open(wav_path).context("Failed to open WAV file")?;
    let spec = reader.spec();
    let sr = spec.sample_rate as usize;
    let samples: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Int => reader
            .samples::<i16>()
            .map(|s| s.unwrap_or(0) as f32 / i16::MAX as f32)
            .collect(),
        hound::SampleFormat::Float => reader
            .samples::<f32>()
            .map(|s| s.unwrap_or(0.0))
            .collect(),
    };

    let duration = samples.len() as f64 / sr as f64;
    log::info!("Loaded audio: {:.2}s at {}Hz", duration, sr);

    // 2. Voice Activity Detection
    let speech_segments = detect_speech(&samples, sr);
    log::info!("Detected {} speech segments", speech_segments.len());

    if speech_segments.is_empty() {
        log::warn!("No speech detected in audio");
        return Ok(vec![]);
    }

    // 3. Extract MFCC features
    let (embeddings, segment_info) = extract_features(&samples, sr, &speech_segments);

    if segment_info.is_empty() {
        log::warn!("No valid features extracted");
        return Ok(vec![]);
    }

    // 4. Normalize features (z-score)
    let embeddings = normalize_features(&embeddings);

    // 5. Estimate number of speakers
    let num_speakers =
        estimate_num_speakers(&embeddings, min_speakers, max_speakers.min(segment_info.len()));
    log::info!("Estimated {} speakers", num_speakers);

    // 6. Cluster speakers
    let labels = cluster_speakers(&embeddings, num_speakers);

    // 7. Create speaker segments
    let mut segments: Vec<DiarizationSegment> = segment_info
        .iter()
        .zip(labels.iter())
        .map(|(seg, &label)| DiarizationSegment {
            speaker: format!("SPEAKER_{:02}", label),
            start: seg.start,
            end: seg.end,
        })
        .collect();

    segments.sort_by(|a, b| a.start.partial_cmp(&b.start).unwrap());

    // 8. Post-process: merge adjacent same-speaker segments
    let segments = post_process(segments);

    log::info!("Diarization complete: {} segments", segments.len());
    Ok(segments)
}

/// Energy-based Voice Activity Detection
fn detect_speech(samples: &[f32], sr: usize) -> Vec<SpeechSegment> {
    let frame_len = (FRAME_LENGTH * sr as f64) as usize;
    let hop_len = (FRAME_SHIFT * sr as f64) as usize;

    if samples.len() < frame_len {
        return vec![];
    }

    // Compute frame-level RMS energy
    let num_frames = (samples.len() - frame_len) / hop_len + 1;
    let mut energy = Vec::with_capacity(num_frames);

    for i in 0..num_frames {
        let start = i * hop_len;
        let end = (start + frame_len).min(samples.len());
        let frame = &samples[start..end];

        let rms = (frame.iter().map(|&s| s * s).sum::<f32>() / frame.len() as f32).sqrt();
        energy.push(rms);
    }

    // Normalize energy to [0, 1]
    let min_e = energy.iter().cloned().fold(f32::INFINITY, f32::min);
    let max_e = energy.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let range = max_e - min_e + 1e-10;

    let energy_norm: Vec<f32> = energy.iter().map(|&e| (e - min_e) / range).collect();

    // Apply threshold
    let total_dur = samples.len() as f64 / sr as f64;
    let mut segments = Vec::new();
    let mut in_speech = false;
    let mut speech_start = 0.0;

    for (i, &e) in energy_norm.iter().enumerate() {
        let time_sec = i as f64 * hop_len as f64 / sr as f64;
        let is_speech = e > VAD_THRESHOLD;

        if is_speech && !in_speech {
            in_speech = true;
            speech_start = (time_sec - VAD_PAD_DUR).max(0.0);
        } else if !is_speech && in_speech {
            in_speech = false;
            let speech_end = (time_sec + VAD_PAD_DUR).min(total_dur);
            if speech_end - speech_start >= MIN_SEGMENT_DUR {
                segments.push(SpeechSegment {
                    start: speech_start,
                    end: speech_end,
                });
            }
        }
    }

    // Handle audio ending during speech
    if in_speech {
        let speech_end = total_dur;
        if speech_end - speech_start >= MIN_SEGMENT_DUR {
            segments.push(SpeechSegment {
                start: speech_start,
                end: speech_end,
            });
        }
    }

    segments
}

/// Extract MFCC features from speech segments
fn extract_features(
    samples: &[f32],
    sr: usize,
    segments: &[SpeechSegment],
) -> (Array2<f64>, Vec<SpeechSegment>) {
    let mut all_embeddings: Vec<Vec<f64>> = Vec::new();
    let mut valid_segments: Vec<SpeechSegment> = Vec::new();

    for seg in segments {
        let start_sample = (seg.start * sr as f64) as usize;
        let end_sample = ((seg.end * sr as f64) as usize).min(samples.len());
        let segment_audio = &samples[start_sample..end_sample];

        if segment_audio.len() < (sr as f64 * MIN_SEGMENT_DUR) as usize {
            continue;
        }

        // Compute MFCCs for this segment
        let mfccs = compute_mfcc(segment_audio, sr);

        if mfccs.is_empty() {
            continue;
        }

        // Compute deltas
        let delta = compute_delta(&mfccs);
        let delta2 = compute_delta(&delta);

        // Combine: for each frame, concat MFCC + delta + delta2 = 60 features
        let n_frames = mfccs.len();
        let mut frame_features: Vec<Vec<f64>> = Vec::with_capacity(n_frames);

        for i in 0..n_frames {
            let mut feat = Vec::with_capacity(N_MFCC * 3);
            feat.extend_from_slice(&mfccs[i]);
            feat.extend_from_slice(&delta[i]);
            feat.extend_from_slice(&delta2[i]);
            frame_features.push(feat);
        }

        // Compute segment embedding: mean of all frame features
        let feat_dim = N_MFCC * 3;
        let mut embedding = vec![0.0f64; feat_dim];
        for frame in &frame_features {
            for (j, &val) in frame.iter().enumerate() {
                embedding[j] += val;
            }
        }
        for val in &mut embedding {
            *val /= n_frames as f64;
        }

        all_embeddings.push(embedding);
        valid_segments.push(seg.clone());
    }

    if all_embeddings.is_empty() {
        return (Array2::zeros((0, 0)), vec![]);
    }

    let n = all_embeddings.len();
    let d = all_embeddings[0].len();
    let flat: Vec<f64> = all_embeddings.into_iter().flatten().collect();
    let embeddings = Array2::from_shape_vec((n, d), flat).unwrap();

    (embeddings, valid_segments)
}

/// Compute MFCC features for a segment of audio
/// Returns Vec of frames, each frame has N_MFCC coefficients
fn compute_mfcc(samples: &[f32], sr: usize) -> Vec<Vec<f64>> {
    let frame_len = (FRAME_LENGTH * sr as f64) as usize;
    let hop_len = (FRAME_SHIFT * sr as f64) as usize;

    if samples.len() < frame_len {
        return vec![];
    }

    let n_frames = (samples.len() - frame_len) / hop_len + 1;

    // Pre-emphasis
    let mut emphasized = vec![0.0f64; samples.len()];
    emphasized[0] = samples[0] as f64;
    for i in 1..samples.len() {
        emphasized[i] = samples[i] as f64 - 0.97 * samples[i - 1] as f64;
    }

    // FFT size (next power of 2)
    let n_fft = frame_len.next_power_of_two();

    // Hamming window
    let window: Vec<f64> = (0..frame_len)
        .map(|i| 0.54 - 0.46 * (2.0 * std::f64::consts::PI * i as f64 / (frame_len - 1) as f64).cos())
        .collect();

    // Build mel filterbank
    let mel_filters = mel_filterbank(n_fft, sr, N_MELS);

    // DCT matrix for MFCC
    let dct_matrix = dct_matrix(N_MFCC, N_MELS);

    let mut planner = rustfft::FftPlanner::<f64>::new();
    let fft = planner.plan_fft_forward(n_fft);

    let mut mfccs: Vec<Vec<f64>> = Vec::with_capacity(n_frames);

    for i in 0..n_frames {
        let start = i * hop_len;
        let end = (start + frame_len).min(emphasized.len());

        // Apply window
        let mut frame: Vec<rustfft::num_complex::Complex<f64>> = (0..n_fft)
            .map(|j| {
                if j < end - start {
                    rustfft::num_complex::Complex::new(emphasized[start + j] * window[j], 0.0)
                } else {
                    rustfft::num_complex::Complex::new(0.0, 0.0)
                }
            })
            .collect();

        // FFT
        fft.process(&mut frame);

        // Power spectrum (only first half + 1)
        let power_len = n_fft / 2 + 1;
        let power_spec: Vec<f64> = frame[..power_len]
            .iter()
            .map(|c| c.norm_sqr() / n_fft as f64)
            .collect();

        // Apply mel filterbank
        let mut mel_energies = vec![0.0f64; N_MELS];
        for (m, filter) in mel_filters.iter().enumerate() {
            for (k, &weight) in filter.iter().enumerate() {
                if k < power_spec.len() {
                    mel_energies[m] += weight * power_spec[k];
                }
            }
        }

        // Log mel energies
        for e in &mut mel_energies {
            *e = (*e + 1e-10).ln();
        }

        // DCT to get MFCCs
        let mut mfcc = vec![0.0f64; N_MFCC];
        for (i, coeff) in mfcc.iter_mut().enumerate() {
            for (j, &mel) in mel_energies.iter().enumerate() {
                *coeff += dct_matrix[i][j] * mel;
            }
        }

        mfccs.push(mfcc);
    }

    mfccs
}

/// Compute delta features (first-order differences)
fn compute_delta(features: &[Vec<f64>]) -> Vec<Vec<f64>> {
    let n = features.len();
    if n < 3 {
        return features.to_vec();
    }

    let width = 2;
    let denom: f64 = 2.0 * (1..=width).map(|i| (i * i) as f64).sum::<f64>();

    let dim = features[0].len();
    let mut delta = vec![vec![0.0f64; dim]; n];

    for t in 0..n {
        for d in 0..dim {
            let mut val = 0.0;
            for i in 1..=width {
                let prev = if t >= i { features[t - i][d] } else { features[0][d] };
                let next = if t + i < n {
                    features[t + i][d]
                } else {
                    features[n - 1][d]
                };
                val += i as f64 * (next - prev);
            }
            delta[t][d] = val / denom;
        }
    }

    delta
}

/// Build a mel filterbank matrix
fn mel_filterbank(n_fft: usize, sr: usize, n_mels: usize) -> Vec<Vec<f64>> {
    let power_len = n_fft / 2 + 1;

    let mel_low = hz_to_mel(0.0);
    let mel_high = hz_to_mel(sr as f64 / 2.0);

    // Mel points evenly spaced
    let mel_points: Vec<f64> = (0..n_mels + 2)
        .map(|i| mel_low + (mel_high - mel_low) * i as f64 / (n_mels + 1) as f64)
        .collect();

    // Convert mel points to Hz, then to FFT bin indices
    let hz_points: Vec<f64> = mel_points.iter().map(|&m| mel_to_hz(m)).collect();
    let bin_points: Vec<usize> = hz_points
        .iter()
        .map(|&hz| ((n_fft as f64 + 1.0) * hz / sr as f64).floor() as usize)
        .collect();

    let mut filters = vec![vec![0.0f64; power_len]; n_mels];

    for m in 0..n_mels {
        let left = bin_points[m];
        let center = bin_points[m + 1];
        let right = bin_points[m + 2];

        for k in left..center {
            if center > left && k < power_len {
                filters[m][k] = (k - left) as f64 / (center - left) as f64;
            }
        }
        for k in center..right {
            if right > center && k < power_len {
                filters[m][k] = (right - k) as f64 / (right - center) as f64;
            }
        }
    }

    filters
}

fn hz_to_mel(hz: f64) -> f64 {
    2595.0 * (1.0 + hz / 700.0).log10()
}

fn mel_to_hz(mel: f64) -> f64 {
    700.0 * (10.0_f64.powf(mel / 2595.0) - 1.0)
}

/// DCT-II matrix for MFCC computation
fn dct_matrix(n_mfcc: usize, n_mels: usize) -> Vec<Vec<f64>> {
    let mut matrix = vec![vec![0.0f64; n_mels]; n_mfcc];

    for i in 0..n_mfcc {
        for j in 0..n_mels {
            matrix[i][j] = (std::f64::consts::PI * i as f64 * (j as f64 + 0.5) / n_mels as f64).cos();
        }
    }

    matrix
}

/// Z-score normalization per feature
fn normalize_features(embeddings: &Array2<f64>) -> Array2<f64> {
    let n = embeddings.nrows();
    if n <= 1 {
        return embeddings.clone();
    }

    let mean = embeddings.mean_axis(Axis(0)).unwrap();
    let std_dev = embeddings.std_axis(Axis(0), 0.0);

    let mut normalized = embeddings.clone();
    for mut row in normalized.rows_mut() {
        for (j, val) in row.iter_mut().enumerate() {
            let sd = std_dev[j];
            if sd > 1e-10 {
                *val = (*val - mean[j]) / sd;
            } else {
                *val = 0.0;
            }
        }
    }

    normalized
}

/// Estimate optimal number of speakers using elbow method
fn estimate_num_speakers(
    embeddings: &Array2<f64>,
    min_speakers: usize,
    max_speakers: usize,
) -> usize {
    let n = embeddings.nrows();

    if n <= min_speakers {
        return min_speakers;
    }
    if min_speakers == max_speakers {
        return min_speakers;
    }

    let max_k = max_speakers.min(n);
    let mut distortions = Vec::new();

    for k in min_speakers..=max_k {
        let labels = cluster_speakers(embeddings, k);
        let mut distortion = 0.0;

        for cluster_id in 0..k {
            let cluster_indices: Vec<usize> = labels
                .iter()
                .enumerate()
                .filter(|(_, &l)| l == cluster_id)
                .map(|(i, _)| i)
                .collect();

            if cluster_indices.is_empty() {
                continue;
            }

            // Compute centroid
            let dim = embeddings.ncols();
            let mut centroid = vec![0.0f64; dim];
            for &idx in &cluster_indices {
                for d in 0..dim {
                    centroid[d] += embeddings[[idx, d]];
                }
            }
            for val in &mut centroid {
                *val /= cluster_indices.len() as f64;
            }

            // Sum of squared distances to centroid
            for &idx in &cluster_indices {
                let mut dist_sq = 0.0;
                for d in 0..dim {
                    let diff = embeddings[[idx, d]] - centroid[d];
                    dist_sq += diff * diff;
                }
                distortion += dist_sq;
            }
        }

        distortions.push(distortion);
    }

    if distortions.len() <= 1 {
        return min_speakers;
    }

    // Find elbow using successive differences
    let diffs: Vec<f64> = distortions.windows(2).map(|w| w[0] - w[1]).collect();
    let max_diff = diffs.iter().cloned().fold(0.0f64, f64::max);

    if max_diff > 0.0 {
        let normalized: Vec<f64> = diffs.iter().map(|&d| d / max_diff).collect();

        for (i, &diff) in normalized.iter().enumerate() {
            if diff < 0.2 {
                return min_speakers + i + 1;
            }
        }
    }

    2.max(min_speakers).min(max_speakers)
}

/// Agglomerative clustering with Ward linkage
pub fn cluster_speakers(embeddings: &Array2<f64>, n_clusters: usize) -> Vec<usize> {
    let n = embeddings.nrows();

    if n <= 1 {
        return vec![0; n];
    }
    if n_clusters >= n {
        return (0..n).collect();
    }

    // Track which points belong to each cluster
    let mut cluster_members: Vec<Vec<usize>> = (0..n).map(|i| vec![i]).collect();
    let mut active_clusters: Vec<usize> = (0..n).collect();

    // Compute initial pairwise distance matrix
    let dim = embeddings.ncols();

    while active_clusters.len() > n_clusters {
        // Find the pair of clusters that minimizes Ward's criterion
        let mut best_i = 0;
        let mut best_j = 1;
        let mut best_ward = f64::INFINITY;

        for ai in 0..active_clusters.len() {
            for aj in (ai + 1)..active_clusters.len() {
                let ci = active_clusters[ai];
                let cj = active_clusters[aj];

                let members_i = &cluster_members[ci];
                let members_j = &cluster_members[cj];
                let ni = members_i.len() as f64;
                let nj = members_j.len() as f64;

                // Ward's criterion: (ni * nj) / (ni + nj) * ||ci - cj||^2
                // where ci, cj are centroids
                let mut centroid_i = vec![0.0f64; dim];
                let mut centroid_j = vec![0.0f64; dim];

                for &idx in members_i {
                    for d in 0..dim {
                        centroid_i[d] += embeddings[[idx, d]];
                    }
                }
                for val in &mut centroid_i {
                    *val /= ni;
                }

                for &idx in members_j {
                    for d in 0..dim {
                        centroid_j[d] += embeddings[[idx, d]];
                    }
                }
                for val in &mut centroid_j {
                    *val /= nj;
                }

                let mut dist_sq = 0.0;
                for d in 0..dim {
                    let diff = centroid_i[d] - centroid_j[d];
                    dist_sq += diff * diff;
                }

                let ward = (ni * nj) / (ni + nj) * dist_sq;

                if ward < best_ward {
                    best_ward = ward;
                    best_i = ai;
                    best_j = aj;
                }
            }
        }

        // Merge clusters
        let ci = active_clusters[best_i];
        let cj = active_clusters[best_j];

        let members_j = cluster_members[cj].clone();
        cluster_members[ci].extend(members_j);
        cluster_members.push(vec![]); // placeholder

        // Remove cj from active clusters (remove larger index first)
        active_clusters.remove(best_j);
    }

    // Assign final labels
    let mut labels = vec![0usize; n];
    for (label, &cluster_id) in active_clusters.iter().enumerate() {
        for &member in &cluster_members[cluster_id] {
            labels[member] = label;
        }
    }

    labels
}

/// Post-process: merge adjacent segments from the same speaker
fn post_process(segments: Vec<DiarizationSegment>) -> Vec<DiarizationSegment> {
    if segments.is_empty() {
        return vec![];
    }

    let mut merged = Vec::new();
    let mut current = segments[0].clone();

    for seg in segments.iter().skip(1) {
        if seg.speaker == current.speaker && seg.start - current.end < 0.5 {
            current.end = seg.end;
        } else {
            merged.push(current);
            current = seg.clone();
        }
    }
    merged.push(current);

    merged
}
