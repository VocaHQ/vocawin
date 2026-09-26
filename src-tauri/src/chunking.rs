//! Splits a long recording into windows an encoder-decoder model can decode.
//!
//! Canary and Moonshine decode a whole take in one pass. Past about half a
//! minute of real speech they lose text without an error: Canary skips whole
//! sentences and Moonshine repeats a phrase until its token budget runs out.
//! Those models get the take in windows of at most `WINDOW_SECONDS` plus
//! `SEARCH_SECONDS`, cut in the quietest stretch near each even split, and
//! the window texts are joined. Cohere is split the same way because it
//! stops after 512 tokens, and GigaAM because its encoder rejects very long
//! takes. Parakeet and SenseVoice decode takes whole.
//!
//! Windows cover every sample. Deciding up front which audio is silence
//! would sometimes drop quiet speech, so nothing is left out here; a window
//! that decodes to nothing is decoded again `without_long_pauses` instead.

use std::ops::Range;

use crate::silence;

const SAMPLE_RATE: usize = 16_000;
/// 30 ms frames.
const FRAME: usize = 480;
/// Takes up to this long are decoded whole; longer ones are split evenly
/// into windows no longer than this before each cut moves to a pause.
const WINDOW_SECONDS: f32 = 20.0;
/// How far a cut may move from its even split to land in a pause.
const SEARCH_SECONDS: f32 = 2.5;
/// A cut goes in the quietest run of this many frames (150 ms), so a stop
/// consonant inside a word does not pass for a pause.
const QUIET_FRAMES: usize = 5;
/// Frame energy (sum of squares) that always counts as quiet: 0.004 RMS,
/// `silence.rs`'s lowest speech threshold.
const QUIET_FLOOR: f32 = 0.004 * 0.004 * FRAME as f32;
/// Silence a window may open with. Moonshine returns nothing for a window
/// that starts on a second or more of silence, so a cut goes near the end
/// of its pause and the rest of the pause trails the previous window.
const LEAD_SECONDS: f32 = 0.2;
/// `without_long_pauses` measures a window's pause level over its opening
/// this long.
const OPENING_SECONDS: f32 = 0.5;
/// Sound is this many times the opening's RMS (about 10 dB up).
const SOUND_OVER_OPENING: f32 = 3.0;
/// Lowest RMS that counts as sound, `silence.rs`'s minimum.
const MINIMUM_SOUND_RMS: f32 = 0.004;

fn seconds(value: f32) -> usize {
    (value * SAMPLE_RATE as f32) as usize
}

/// Sample ranges to decode in order, covering all of `samples` without gaps.
pub fn windows(samples: &[f32]) -> Vec<Range<usize>> {
    let total = samples.len();
    let window = seconds(WINDOW_SECONDS);
    if total <= window {
        return vec![0..total];
    }
    let count = total.div_ceil(window);
    let search = seconds(SEARCH_SECONDS);
    let mut ranges = Vec::with_capacity(count);
    let mut start = 0;
    for index in 1..count {
        let even = total * index / count;
        let cut = cut_point(
            samples,
            even.saturating_sub(search),
            (even + search).min(total),
        );
        ranges.push(start..cut);
        start = cut;
    }
    ranges.push(start..total);
    ranges
}

/// Where to cut inside `lower..upper`: the pause holding the quietest
/// `QUIET_FRAMES` run, `LEAD_SECONDS` before the pause ends.
fn cut_point(samples: &[f32], lower: usize, upper: usize) -> usize {
    let energies: Vec<f32> = samples[lower..upper]
        .chunks_exact(FRAME)
        .map(|frame| frame.iter().map(|s| s * s).sum::<f32>())
        .collect();
    if energies.len() < QUIET_FRAMES {
        return (lower + upper) / 2;
    }
    let mut best = 0;
    let mut best_energy = f32::MAX;
    for (index, run) in energies.windows(QUIET_FRAMES).enumerate() {
        let energy: f32 = run.iter().sum();
        if energy < best_energy {
            best_energy = energy;
            best = index;
        }
    }
    let quiet = (best_energy / QUIET_FRAMES as f32 * 4.0).max(QUIET_FLOOR);
    let mut pause_end = best + QUIET_FRAMES;
    while pause_end < energies.len() && energies[pause_end] <= quiet {
        pause_end += 1;
    }
    let middle = lower + best * FRAME + QUIET_FRAMES * FRAME / 2;
    (lower + pause_end * FRAME)
        .saturating_sub(seconds(LEAD_SECONDS))
        .max(middle)
}

/// A window's audio with every long pause shortened, the way Skip Silence
/// presents a take: each sound of any length (a word, a click, a hum) is
/// kept with 0.2 s either side, and at most 0.4 s of each pause
/// (`silence::apply`). Moonshine returns nothing for a window that opens on
/// a second or more of pause, and this audio never does.
///
/// A pause is measured from the window's opening, so a noisy room counts
/// as one while sound is clearly louder. After the first clearly-loud
/// frame, quieter speech between louder passages is kept. `None` when
/// nothing would be shortened (no long pause, or no sound at all).
pub fn without_long_pauses(samples: &[f32]) -> Option<Vec<f32>> {
    let rms: Vec<f32> = samples
        .chunks(FRAME)
        .map(|frame| (frame.iter().map(|s| s * s).sum::<f32>() / frame.len() as f32).sqrt())
        .collect();
    let opening_frames = seconds(OPENING_SECONDS) / FRAME;
    if rms.len() <= opening_frames {
        return None;
    }
    let mut opening = rms[..opening_frames].to_vec();
    opening.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let opening_rms = opening[opening.len() / 2];
    let high = (opening_rms * SOUND_OVER_OPENING).max(MINIMUM_SOUND_RMS);
    let first_high = rms.iter().position(|value| *value >= high)?;
    let padding = seconds(LEAD_SECONDS);
    let mut sounds: Vec<Range<usize>> = Vec::new();
    for (index, value) in rms.iter().enumerate() {
        let keep = if index < first_high {
            *value >= high
        } else {
            *value > opening_rms
        };
        if !keep {
            continue;
        }
        let start = (index * FRAME).saturating_sub(padding);
        let end = ((index + 1) * FRAME + padding).min(samples.len());
        match sounds.last_mut() {
            Some(last) if start <= last.end => last.end = end,
            _ => sounds.push(start..end),
        }
    }
    if sounds.is_empty() {
        return None;
    }
    let shortened = silence::apply(&sounds, samples);
    (shortened.len() < samples.len()).then_some(shortened)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tone(seconds_long: f32) -> Vec<f32> {
        (0..seconds(seconds_long))
            .map(|i| (i as f32 * 0.07).sin() * 0.3)
            .collect()
    }

    fn noise(seconds_long: f32, amplitude: f32) -> Vec<f32> {
        (0..seconds(seconds_long))
            .map(|i| if i % 2 == 0 { amplitude } else { -amplitude })
            .collect()
    }

    fn click(samples: &mut [f32], at_seconds: f32) {
        for sample in &mut samples[seconds(at_seconds)..seconds(at_seconds) + FRAME] {
            *sample = 1.0;
        }
    }

    fn assert_covers(ranges: &[Range<usize>], total: usize) {
        assert_eq!(ranges.first().map(|r| r.start), Some(0));
        assert_eq!(ranges.last().map(|r| r.end), Some(total));
        for pair in ranges.windows(2) {
            assert_eq!(pair[0].end, pair[1].start);
        }
    }

    /// Every sample louder than `loud` survives, in order; the audio opens
    /// on at most the 0.2 s kept before a sound, and no quiet stretch is
    /// longer than two paddings and a shortened pause (0.8 s), plus a frame
    /// either side.
    fn assert_keeps_sound_and_shortens_pauses(samples: &[f32], loud: f32) {
        let shortened = without_long_pauses(samples).expect("a long pause to shorten");
        let sound = |audio: &[f32]| -> Vec<f32> {
            audio.iter().copied().filter(|s| s.abs() >= loud).collect()
        };
        assert_eq!(sound(&shortened), sound(samples));
        let opening = shortened.iter().position(|s| s.abs() >= loud).unwrap();
        assert!(
            opening <= seconds(LEAD_SECONDS) + FRAME,
            "opens on {opening} samples"
        );
        let mut quiet = 0;
        let mut longest = 0;
        for sample in &shortened {
            quiet = if sample.abs() < loud { quiet + 1 } else { 0 };
            longest = longest.max(quiet);
        }
        assert!(
            longest <= seconds(0.8) + 2 * FRAME,
            "{longest}-sample pause left"
        );
    }

    #[test]
    fn short_takes_are_decoded_whole() {
        let samples = tone(WINDOW_SECONDS);
        assert_eq!(windows(&samples), vec![0..samples.len()]);
        assert_eq!(windows(&[]), vec![0..0]);
    }

    #[test]
    fn long_takes_split_into_bounded_windows() {
        for length in [20.5, 31.0, 45.0, 60.0, 299.0] {
            let samples = tone(length);
            let ranges = windows(&samples);
            assert_covers(&ranges, samples.len());
            assert_eq!(
                ranges.len(),
                samples.len().div_ceil(seconds(WINDOW_SECONDS))
            );
            for range in &ranges {
                assert!(
                    range.len() <= seconds(WINDOW_SECONDS + 2.0 * SEARCH_SECONDS),
                    "{length}s take has a {}-sample window",
                    range.len()
                );
                assert!(
                    range.len() >= seconds(WINDOW_SECONDS / 2.0 - 2.0 * SEARCH_SECONDS),
                    "{length}s take has a {}-sample window",
                    range.len()
                );
            }
        }
    }

    #[test]
    fn cuts_land_in_a_nearby_pause() {
        // 40 s of speech splits evenly at 20 s; a pause at 18.5 s is in reach.
        let mut samples = tone(18.3);
        samples.extend(vec![0.0; seconds(0.4)]);
        samples.extend(tone(21.3));
        let ranges = windows(&samples);
        assert_eq!(ranges.len(), 2);
        let cut = ranges[0].end;
        assert!(
            cut > seconds(18.3) && cut < seconds(18.7),
            "cut at {:.2}s",
            cut as f32 / SAMPLE_RATE as f32
        );
    }

    #[test]
    fn a_long_pause_trails_the_window_before_it() {
        // Speech resumes at 19 s; the next window opens just before it.
        let mut samples = tone(17.0);
        samples.extend(vec![0.0; seconds(2.0)]);
        samples.extend(tone(19.0));
        let ranges = windows(&samples);
        assert_eq!(ranges.len(), 2);
        let cut = ranges[0].end;
        assert!(
            cut >= seconds(19.0 - LEAD_SECONDS - 0.05) && cut <= seconds(19.0),
            "cut at {:.2}s",
            cut as f32 / SAMPLE_RATE as f32
        );
    }

    #[test]
    fn a_noisy_pause_past_the_search_range_stays_in_the_windows() {
        // An 8 s pause of room noise only 20 dB under the speech covers the
        // whole 17.5-22.5 s search range. No audio is left out: the next
        // window opens on the rest of the pause, and its retry audio has the
        // pause shortened.
        let mut samples = tone(16.0);
        samples.extend(noise(8.0, 0.02));
        samples.extend(tone(16.0));
        let ranges = windows(&samples);
        assert_covers(&ranges, samples.len());
        assert_keeps_sound_and_shortens_pauses(&samples[ranges[1].clone()], 0.1);
    }

    #[test]
    fn quiet_speech_past_a_loud_click_is_never_left_out() {
        let mut samples = tone(17.0);
        samples.extend(
            (0..seconds(9.0))
                .map(|i| (i as f32 * 0.07).sin() * 0.02)
                .collect::<Vec<_>>(),
        );
        samples.extend(tone(14.0));
        click(&mut samples, 5.0);
        assert_covers(&windows(&samples), samples.len());
    }

    #[test]
    fn a_retry_keeps_every_sound_in_the_pause() {
        for level in [0.0, 0.004, 0.02] {
            // A plain lead-in.
            let mut lead = noise(1.5, level);
            lead.extend(tone(3.0));
            assert_keeps_sound_and_shortens_pauses(&lead, 0.1);

            // Clicks early and late in a pause, 1.2 s apart.
            let mut clicks = noise(5.0, level);
            for at in [0.2, 1.4, 2.6, 3.8] {
                click(&mut clicks, at);
            }
            clicks.extend(tone(3.0));
            assert_keeps_sound_and_shortens_pauses(&clicks, 0.1);

            // Short words between long pauses.
            let mut words = noise(1.0, level);
            for _ in 0..4 {
                words.extend(tone(0.06));
                words.extend(noise(1.2, level));
            }
            words.extend(tone(3.0));
            assert_keeps_sound_and_shortens_pauses(&words, 0.1);

            // A sustained non-speech sound (a hum), then the speech.
            let mut hum = noise(1.0, level);
            hum.extend(
                (0..seconds(0.6))
                    .map(|i| (i as f32 * 0.02).sin() * 0.2)
                    .collect::<Vec<_>>(),
            );
            hum.extend(noise(1.5, level));
            hum.extend(tone(3.0));
            assert_keeps_sound_and_shortens_pauses(&hum, 0.1);
        }
    }

    #[test]
    fn nothing_to_shorten_means_no_retry() {
        // Speech right away, quiet speech that gets louder, a short breath
        // first, or no sound at all.
        assert_eq!(without_long_pauses(&tone(3.0)), None);
        let mut rising: Vec<f32> = (0..seconds(2.0))
            .map(|i| (i as f32 * 0.07).sin() * 0.15)
            .collect();
        rising.extend(tone(3.0));
        assert_eq!(without_long_pauses(&rising), None);
        let mut breath = noise(0.2, 0.004);
        breath.extend(tone(3.0));
        assert_eq!(without_long_pauses(&breath), None);
        assert_eq!(without_long_pauses(&noise(3.0, 0.004)), None);
        assert_eq!(without_long_pauses(&[]), None);
    }

    #[test]
    fn quieter_speech_between_louder_passages_survives_a_retry() {
        // Opening room noise 0.02 RMS makes the old single threshold 0.06, which
        // dropped quieter speech between louder passages on retry.
        let mut samples = noise(1.2, 0.02);
        samples.extend(tone(0.8)); // loud ~0.3

        // Quieter sine: peak 0.04, frame RMS ~0.028 (above opening, below 0.06).
        // 1.2 s so 0.2 s padding and a 0.4 s shortened pause cannot reconstruct it.
        samples.extend(
            (0..seconds(1.2))
                .map(|i| (i as f32 * 0.07).sin() * 0.04)
                .collect::<Vec<_>>(),
        );
        samples.extend(tone(0.8));
        samples.extend(noise(1.0, 0.02));

        let shortened = without_long_pauses(&samples).expect("opening pause to shorten");
        let quietish = |audio: &[f32]| -> Vec<f32> {
            audio.iter().copied().filter(|s| s.abs() >= 0.035).collect()
        };
        assert_eq!(quietish(&shortened), quietish(&samples));
        assert!(shortened.len() < samples.len());
        let opening = shortened.iter().position(|s| s.abs() >= 0.035).unwrap();
        assert!(
            opening <= seconds(LEAD_SECONDS) + FRAME,
            "opens on {opening} samples"
        );
    }
}
