# TFinder — light tactile launch film

## Goal

Create a premium 20-second product-launch film for TFinder. It should make a terminal tool feel warm, physical, fast, and unexpectedly delightful while preserving perfectly legible product typography.

## Delivery

- 16:9 master, authored at 1920×1080 and exported at 3840×2160.
- True 60 FPS motion and a 20-second work area.
- H.264 video and 48 kHz AAC audio.
- Editable Diffusion Studio JSX project, final contact sheet, and verified MP4.

## Visual system

- Warm cream background, graphite copy, pastel orange/yellow/blue/mint/lavender accents.
- One continuous spatial world rather than independent slides.
- A single light-theme terminal remains the hero object and changes size/position without losing its identity.
- Rounded matte-plastic keycaps, folder, magnifier, chips, paper cards, soft shadows, tiny highlights, controlled perspective.
- Typography is code-native, never generated into imagery. Product name is always exactly `TFinder`.

## Timeline

1. **0–2 s — Materialize:** small toy developer objects settle into frame; a blinking cursor expands into the terminal.
2. **2–5 s — Introduce:** `$ tf` types, SEARCH/FIND/INSPECT/RUN keycaps spring in, headline: `Your terminal. Faster.`
3. **5–9 s — Search machine:** terminal glides left; folder → magnifier → command → result pipeline works on the right; rhythmic statement: `Find it. Do it. Keep moving.`
4. **9–13 s — Hero:** terminal returns larger; three concise actions complete with tactile reactions; headline: `One command. Less friction.`
5. **13–17 s — Resolve:** surrounding objects converge; terminal collapses into the `>_` brand symbol; TFinder wordmark reveals.
6. **17–20 s — Hold:** `TFinder`, `Terminal work, simplified.`, and `$ tf` hold with subtle drift and two cursor blinks.

## Motion

- 60 FPS, spring settles, small overshoot, overlapping secondary motion, continuous parallax.
- No aggressive cuts, glitch, neon, dark hacker styling, or unreadable code.
- Avoid scale transforms on large HTML wrappers because Diffusion Studio’s canvas rasterizer can blank clipped scaled subtrees; animate width, height, position, and small leaf transforms instead.

## Audio

- Original 120 BPM warm electronic bed: soft kick/pulse, mallet-like plucks, filtered air, tactile transition taps.
- Accents at 2, 5, 9, 13, and 17 seconds; quiet UI clicks sit inside the music.
- No voiceover; the video must work muted.
