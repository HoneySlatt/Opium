# Test data

## `tone.opus`

One second of a 440 Hz sine wave, mono, 48 kHz, in an Ogg Opus file of 2,446 bytes. A test in
`crates/player/src/decoder.rs` (`an_opus_file_decodes_through_libopus`) decodes it.

What the file shows (`ffprobe`):

- Stream tag `encoder=Lavc63.1.102 libopus`: FFmpeg's `libopus` encoder, written by FFmpeg's Ogg
  muxer.
- 48,000 Hz, mono, 312 samples of pre-skip (the encoder's priming), 48,312 samples counted in all.
- A mean rate of about 16 kbit/s. The wave rises through zero 439 times in the second, and its
  peak is about 0.14 of full scale (0.125, the level of FFmpeg's `sine` source, plus the codec's
  overshoot).

The file's origin was not written down when it was added. As far as it shows, it was made with
FFmpeg's `sine` source and `libopus` at 16 kbit/s. This command makes an equivalent file: decoded
with FFmpeg 9.0.2, its samples are the same, bit for bit. The files differ in 32 bytes, the Ogg
stream serial numbers and the checksums that depend on them.

```sh
ffmpeg -f lavfi -i "sine=frequency=440:sample_rate=48000:duration=1" \
  -c:a libopus -b:a 16k -ac 1 tone.opus
```

It holds no third party's work: a generated sine wave, with no recording, sample or tag from
anyone. It is covered by the repository's licence, GPL-3.0-or-later.
