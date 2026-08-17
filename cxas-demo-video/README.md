# cxas-harness demo video (Manim + Manim MCP)

Light-themed, Google-four-color animated product demo of **cxas-harness**,
rendered with Manim (community edition) and set up to be driven through the
**Manim MCP server** (`abhiemj/manim-mcp-server`).

## Files

| Path | What it is |
|---|---|
| `scenes.py` | All 10 scenes (light theme, Google 4 colors, 16:9, 30 fps, 1920x1080) |
| `cxas-harness-demo.mp4` | **Video** — 2:44 (164 s), 1080p30, no audio |
| `cxas-harness-demo-narrated.mp4` | **Narrated video** — 2:44, Sarvam AI voiceover (en-IN, anushka, bulbul:v2) |
| `narration.py` | Per-scene narration text |
| `tts_sarvam.py` | Sarvam TTS + fit/mix pipeline |
| `audio/` | Cached TTS wavs (`*_p1.wav` = natural-pace measure, `*.wav` = paced) |
| `manim-mcp-server/` | The Manim MCP server (cloned from GitHub) |
| `media/` | Manim output (per-scene mp4s, partial renders, frames/) |

## Scene order

1. `TitleScene` — cxas-harness, tagline, badges
2. `PitchScene` — 170/170 · 37 modelled · v1 + v1beta · no "global" default
3. `DefectScene` — the enum-drift bug a green suite missed
4. `ArchitectureScene` — reference → generate → table → core → CES, parity both ways
5. `CoverageScene` — CES-COVERAGE line, v1=66 · v1beta=104 · modelled=37
6. `InstallScene` — build, 221 cargo tests, 58 pytest, clippy clean
7. `CliScene` — typed commands, JSON envelopes, exit codes
8. `AuthStreamingScene` — credential precedence chain (--oauth-token → … → gcloud), token cache/refresh, streamRunSession message-by-message semantics
9. `GauntletScene` — Builder → evidence.py → Blind critic loop + stop conditions
10. `OutroScene` — links, license

## Voiceover (Sarvam AI)

```bash
# 1. install deps (once)
.venv/Scripts/python -m pip install manim

# 2. generate + mix (needs the API key in the environment only)
SARVAM_API_KEY=sk_... .venv/Scripts/python tts_sarvam.py
# -> cxas-harness-demo-narrated.mp4
```

- Model `bulbul:v2`, lang `en-IN`, speaker `anushka` — change `SPEAKER` in `tts_sarvam.py`
- Per-scene target pace fills ~75% of each scene; natural-pace clips are cached
  as `audio/*_p1.wav`, paced clips as `audio/*.wav` (keyed by a `.pace` marker)
- Final mix: each clip fitted to its scene (atempo if needed, else silence-pad),
  concatenated, then `loudnorm=I=-16:TP=-1.5:LRA=11`, muxed as AAC 192k
- The API key never touches a file; keep it out of git

## Text-fit guard

`fit()` in `scenes.py` scales any text down so it stays inside its box or on
frame — applied to every label. `measure_text.py` prints measured widths if you
need to tune sizes by hand.

## Pacing

`PACING` at the top of `scenes.py` scales every animation `run_time` and `wait`
globally. Current value `1.8`; lower it for a snappier cut, raise it for a
slower one. Changing it invalidates the render cache for every scene.

## Re-render everything

```bash
.venv/Scripts/python -m pip install manim   # once
.venv/Scripts/manim -qh scenes.py TitleScene PitchScene DefectScene \
    ArchitectureScene CoverageScene InstallScene CliScene AuthStreamingScene \
    GauntletScene OutroScene
```

Renders are cached per scene, so re-rendering one scene after an edit is fast:

```bash
.venv/Scripts/manim -qh scenes.py CliScene
```

## Rebuild the concatenated video

```bash
rm -f concat.txt cxas-harness-demo.mp4
for s in TitleScene PitchScene DefectScene ArchitectureScene CoverageScene \
         InstallScene CliScene AuthStreamingScene GauntletScene OutroScene; do
  echo "file 'media/videos/scenes/1080p30/$s.mp4'" >> concat.txt
done
ffmpeg -y -f concat -safe 0 -i concat.txt -c copy cxas-harness-demo.mp4
```

## Drive it through the Manim MCP server

The server (`manim-mcp-server/src/manim_server.py`) exposes an
`execute_manim_code` tool that writes your Manim script and renders it. Add it
to an MCP host — Claude Desktop:

```json
{
  "mcpServers": {
    "manim-server": {
      "command": "C:/Users/yashk/Downloads/cxas-harness/cxas-demo-video/.venv/Scripts/python.exe",
      "args": [
        "C:/Users/yashk/Downloads/cxas-harness/cxas-demo-video/manim-mcp-server/src/manim_server.py"
      ],
      "env": {
        "MANIM_EXECUTABLE": "C:/Users/yashk/Downloads/cxas-harness/cxas-demo-video/.venv/Scripts/manim.exe"
      }
    }
  }
}
```

For Cursor / Claude Code, the same command + args go in your MCP config.
Then tell the host: *"render a scene with execute_manim_code, light theme,
Google colors, and save it in media/videos"*.

> Note: the server's render command passes `-p` (open preview). For headless
> CI renders use the `manim` CLI directly as shown above.
