#!/usr/bin/env python3
"""Makes a throwaway profile holding a library of the owner's library's size, the same every time.

usage: scripts/measure/library.py <profile-dir> [songs]

The same arguments and the same ffmpeg always give the same files, so that the egui window's
figures and uwui's are taken over the same songs; every random choice comes from one seeded
generator, in the main thread. Writes, under <profile-dir>:
- music/<artist>/<album>/<nn> <title>.flac: songs of silence, two to seven minutes long, tagged
  (title, artist, album, album artist, track, year, genre), about 10 kB each through FLAC's
  longest frames; 8,500 songs by default, in 647 albums by 194 artists;
- music/<artist>/<album>/cover.jpg: one cover an album, 1200x1200, a gradient between two colours
  with a seeded grain, about 380 kB, so that each decodes as a photograph's would;
- config/opium/settings.toml: the music folder, the interface at 150 % (`ui_scale = 1.5`, which
  draws at the owner's 1.5 with a display at scale 1), muted.
The app reads the folder at its first start and keeps an index in cache/opium/local: a first run
is a scan, the runs after it read the library from that index, as every start but the first does.
Needs ffmpeg, from `nix develop`. Refuses a profile folder that already holds anything.
"""
import concurrent.futures
import json
import math
import os
import random
import subprocess
import sys
import tempfile

profile = os.path.abspath(sys.argv[1])
songs_wanted = int(sys.argv[2]) if len(sys.argv) > 2 else 8500
if os.path.exists(profile) and os.listdir(profile):
    sys.exit(f"{profile} already holds something: give a new or empty folder")
music = os.path.join(profile, "music")
config = os.path.join(profile, "config", "opium")
os.makedirs(music, exist_ok=True)
os.makedirs(config, exist_ok=True)
os.makedirs(os.path.join(profile, "cache"), exist_ok=True)
os.makedirs(os.path.join(profile, "data"), exist_ok=True)

rng = random.Random(1_14)
SYLLABLES = ["la", "mor", "ve", "si", "tan", "ka", "rho", "len", "da", "mi", "sol", "fé", "nu",
             "bel", "zor", "ia", "qua", "rë", "on", "ys", "dü", "pa", "cel", "ti", "wen", "ost"]
WORDS = ["night", "river", "glass", "gold", "slow", "light", "echo", "fever", "blue", "summer",
         "ghost", "honey", "static", "wild", "heart", "dream", "neon", "storm", "velvet", "home",
         "after", "falling", "under", "the", "of", "in", "away", "again", "never", "only"]
GENRES = ["Pop", "Rock", "Hip-Hop/Rap", "Electronic", "R&B/Soul", "Alternative", "Jazz",
          "Classical", "Dance", "Indie Pop", "Soundtrack", "Metal", "Folk", "K-Pop", "Ambient"]


def name():
    return "".join(rng.choice(SYLLABLES) for _ in range(rng.randint(2, 4))).capitalize()


def title(longest):
    words = [rng.choice(WORDS) for _ in range(rng.randint(1, longest))]
    return " ".join(words).capitalize()


artists = []
for _ in range(200):
    artist = name() if rng.random() < 0.6 else f"{name()} {name()}"
    artists.append(artist)

albums = []
songs = 0
while songs < songs_wanted:
    count = min(rng.randint(6, 20), songs_wanted - songs)
    albums.append({
        "artist": rng.choice(artists),
        "title": title(4),
        "year": rng.randint(1965, 2026),
        "genre": rng.choice(GENRES),
        "tracks": [(title(7), rng.randrange(120, 420, 10)) for _ in range(count)],
        # The cover: a gradient between two colours, at an angle.
        "colours": [[rng.randrange(256) for _ in range(3)] for _ in range(2)],
        "angle": rng.uniform(0, 6.283),
    })
    songs += count

# One silent FLAC a length, copied with each song's tags.
templates = tempfile.mkdtemp(prefix="opium-library.")
lengths = sorted({length for album in albums for _, length in album["tracks"]})


def run(args):
    subprocess.run(["ffmpeg", "-v", "error", "-y", *args], check=True, stdin=subprocess.DEVNULL)


def template(length):
    path = os.path.join(templates, f"{length}.flac")
    run(["-f", "lavfi", "-i", "anullsrc=r=44100:cl=stereo", "-t", str(length), "-c:a", "flac",
         "-frame_size", "65535", path])


def album_files(index, album):
    folder = os.path.join(music, album["artist"], f'{album["title"]} ({index})')
    os.makedirs(folder, exist_ok=True)
    (r1, g1, b1), (r2, g2, b2) = album["colours"]
    along = f"clip(0.5+((X/W-0.5)*{math.cos(album['angle']):.4f}+(Y/H-0.5)*{math.sin(album['angle']):.4f}),0,1)"
    gradient = f"geq=r='{r1}+({r2 - r1})*{along}':g='{g1}+({g2 - g1})*{along}':b='{b1}+({b2 - b1})*{along}'"
    run(["-f", "lavfi", "-i",
         f"color=black:s=1200x1200,format=rgb24,{gradient},format=yuv420p,"
         f"noise=c0s=24:c0_seed={index},gblur=sigma=0.5",
         "-frames:v", "1", "-q:v", "3", os.path.join(folder, "cover.jpg")])
    for number, (song, length) in enumerate(album["tracks"], start=1):
        tags = {"title": song, "artist": album["artist"], "album_artist": album["artist"],
                "album": album["title"], "track": str(number), "date": str(album["year"]),
                "genre": album["genre"]}
        metadata = [arg for key, value in tags.items() for arg in ("-metadata", f"{key}={value}")]
        run(["-i", os.path.join(templates, f"{length}.flac"), "-c", "copy", *metadata,
             os.path.join(folder, f"{number:02} {song}.flac")])


with concurrent.futures.ThreadPoolExecutor(os.cpu_count()) as pool:
    list(pool.map(template, lengths))
    list(pool.map(album_files, range(len(albums)), albums))
for length in lengths:
    os.remove(os.path.join(templates, f"{length}.flac"))
os.rmdir(templates)

with open(os.path.join(config, "settings.toml"), "w") as settings:
    # A JSON string is a TOML basic string.
    settings.write(f"local_folders = [{json.dumps(music)}]\nui_scale = 1.5\nmuted = true\n")
print(f"{songs} songs, {len(albums)} albums, {len(set(a['artist'] for a in albums))} artists in {music}")
