use crate::timer::PomodoroPhase;
use std::io::Cursor;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Sender};
use std::sync::{Mutex, OnceLock};

static AUDIO_MUTED: AtomicBool = AtomicBool::new(false);
static AUDIO_WORKER: OnceLock<Mutex<Option<Sender<Vec<u8>>>>> = OnceLock::new();

pub fn set_audio_muted(muted: bool) {
    AUDIO_MUTED.store(muted, Ordering::SeqCst);
}

fn spawn_worker() -> Sender<Vec<u8>> {
    let (tx, rx) = mpsc::channel::<Vec<u8>>();
    let _ = std::thread::Builder::new()
        .name("tomatui-audio".to_string())
        .spawn(move || {
            let Ok((_stream, handle)) = rodio::OutputStream::try_default() else {
                for _ in rx {}
                return;
            };
            for wav in rx {
                if let Ok(source) = rodio::Decoder::new(Cursor::new(wav)) {
                    if let Ok(sink) = rodio::Sink::try_new(&handle) {
                        sink.append(source);
                        sink.sleep_until_end();
                    }
                }
            }
        });
    tx
}

fn worker_sender() -> Sender<Vec<u8>> {
    let cell = AUDIO_WORKER.get_or_init(|| Mutex::new(None));
    let mut slot = cell
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(tx) = slot.as_ref() {
        return tx.clone();
    }
    let tx = spawn_worker();
    *slot = Some(tx.clone());
    tx
}

pub fn encode_wav_pcm16(samples: &[i16], sample_rate: u32) -> Vec<u8> {
    let mut buf = Vec::with_capacity(44 + samples.len() * 2);
    let data_bytes = (samples.len() * 2) as u32;
    let chunk_size = 36 + data_bytes;
    let byte_rate = sample_rate * 2;

    buf.extend_from_slice(b"RIFF");
    buf.extend_from_slice(&chunk_size.to_le_bytes());
    buf.extend_from_slice(b"WAVE");
    buf.extend_from_slice(b"fmt ");
    buf.extend_from_slice(&16u32.to_le_bytes());
    buf.extend_from_slice(&1u16.to_le_bytes());
    buf.extend_from_slice(&1u16.to_le_bytes());
    buf.extend_from_slice(&sample_rate.to_le_bytes());
    buf.extend_from_slice(&byte_rate.to_le_bytes());
    buf.extend_from_slice(&2u16.to_le_bytes());
    buf.extend_from_slice(&16u16.to_le_bytes());
    buf.extend_from_slice(b"data");
    buf.extend_from_slice(&data_bytes.to_le_bytes());
    for &s in samples {
        buf.extend_from_slice(&s.to_le_bytes());
    }
    buf
}

fn fade_envelope(t: f32, len: f32) -> f32 {
    let fade = 0.01;
    let attack = (t / 0.005).clamp(0.0, 1.0);
    let tail = ((len - t) / fade).clamp(0.0, 1.0);
    attack * tail
}

pub fn synthesize_chime(phase: PomodoroPhase) -> Vec<u8> {
    let sr = 44100u32;
    let (dur, notes): (f32, &[(f32, f32, f32)]) = match phase {
        // Warm bell tone (528 Hz)
        PomodoroPhase::Work => (1.5, &[(0.0, 1.5, 528.0)]),
        // Crisp ascending two-note chime (D5 -> A5)
        PomodoroPhase::ShortBreak => (1.1, &[(0.0, 0.22, 587.33), (0.22, 0.88, 880.0)]),
        // Calming triad (C5 -> E5 -> G5)
        PomodoroPhase::LongBreak => (
            1.6,
            &[(0.0, 0.20, 523.25), (0.20, 0.20, 659.25), (0.40, 1.20, 783.99)],
        ),
    };

    let total = (sr as f32 * dur) as usize;
    let mut samples = Vec::with_capacity(total);
    for i in 0..total {
        let t = i as f32 / sr as f32;
        let mut val = 0.0f32;
        for &(start, len, freq) in notes {
            if t >= start && t < start + len {
                let nt = t - start;
                let decay = (-3.0 * nt).exp();
                let fundamental = (2.0 * std::f32::consts::PI * freq * nt).sin() * 0.72;
                let overtone = (2.0 * std::f32::consts::PI * freq * 2.0 * nt).sin() * 0.20;
                val += (fundamental + overtone) * decay * fade_envelope(nt, len);
            }
        }
        let sample = (val * 26000.0).clamp(i16::MIN as f32, i16::MAX as f32) as i16;
        samples.push(sample);
    }
    encode_wav_pcm16(&samples, sr)
}

pub fn play_phase_chime(phase: PomodoroPhase) {
    if AUDIO_MUTED.load(Ordering::SeqCst) {
        return;
    }
    let wav = synthesize_chime(phase);
    let tx = worker_sender();
    if tx.send(wav).is_err() {
        if let Some(cell) = AUDIO_WORKER.get() {
            if let Ok(mut slot) = cell.lock() {
                *slot = None;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_synthesized_chimes_valid_wav() {
        for phase in [
            PomodoroPhase::Work,
            PomodoroPhase::ShortBreak,
            PomodoroPhase::LongBreak,
        ] {
            let wav = synthesize_chime(phase);
            assert!(wav.len() > 44);
            assert_eq!(&wav[0..4], b"RIFF");
            assert_eq!(&wav[8..12], b"WAVE");
            assert!(rodio::Decoder::new(Cursor::new(wav)).is_ok());
        }
    }
}
