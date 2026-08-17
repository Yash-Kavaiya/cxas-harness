"""Generate per-scene narration via Sarvam AI TTS and mix it into the video.

Usage:
    SARVAM_API_KEY=sk_... .venv/Scripts/python tts_sarvam.py

Steps:
  1. For each scene: call Sarvam /text-to-speech (bulbul:v2, en-IN) and cache
     the wav under audio/.
  2. Fit each clip to its scene duration (speed up if needed, pad with
     silence), concatenate into one narration track, loudnorm, then mux with
     the video.
"""
import base64
import json
import os
import subprocess
import sys
import urllib.request
from pathlib import Path

HERE = Path(__file__).resolve().parent
AUDIO_DIR = HERE / "audio"
AUDIO_DIR.mkdir(exist_ok=True)

API_URL = "https://api.sarvam.ai/text-to-speech"
SPEAKER = "anushka"
MODEL = "bulbul:v2"
LANG = "en-IN"
SAMPLE_RATE = 24000

sys.path.insert(0, str(HERE))
from narration import NARRATION  # noqa: E402

SCENES = [
    "TitleScene", "PitchScene", "DefectScene", "ArchitectureScene",
    "CoverageScene", "InstallScene", "CliScene", "AuthStreamingScene",
    "GauntletScene", "OutroScene",
]
VIDEO = HERE / "cxas-harness-demo.mp4"
OUT_VIDEO = HERE / "cxas-harness-demo-narrated.mp4"


def tts(text: str, key: str, pace: float) -> bytes:
    body = json.dumps({
        "inputs": [text],
        "target_language_code": LANG,
        "speaker": SPEAKER,
        "model": MODEL,
        "pitch": 0,
        "pace": pace,
        "audio_format": "wav",
        "speech_sample_rate": SAMPLE_RATE,
    }).encode()
    req = urllib.request.Request(
        API_URL, data=body,
        headers={"api-subscription-key": key, "Content-Type": "application/json"},
    )
    with urllib.request.urlopen(req, timeout=120) as resp:
        data = json.loads(resp.read().decode())
    audios = data.get("audios") or []
    if not audios:
        raise RuntimeError(f"Sarvam returned no audio: {data}")
    return base64.b64decode(audios[0])


def target_pace(scene: str, scene_dur: float) -> float:
    """Pick a Sarvam pace so narration fills ~75% of the scene.

    pace 1.0 = normal; <1.0 is slower (longer). Clamped to keep speech
    natural (0.62 .. 1.2).
    """
    wav = AUDIO_DIR / f"{scene}_p1.wav"
    if wav.exists():
        base_dur = ffprobe_duration(wav)
    else:
        base_dur = None
    if base_dur and base_dur > 0:
        return max(0.62, min(1.2, base_dur / (scene_dur * 0.75)))
    return 1.0


def ffprobe_duration(path: Path) -> float:
    out = subprocess.run(
        ["ffprobe", "-v", "error", "-show_entries", "format=duration",
         "-of", "csv=p=0", str(path)],
        capture_output=True, text=True,
    ).stdout.strip()
    return float(out)


def main():
    key = os.environ.get("SARVAM_API_KEY", "").strip()
    if not key:
        print("SARVAM_API_KEY is required")
        return 1

    # 1. generate / cache audio
    # First pass at pace 1.0 to measure natural length, cached separately.
    for scene in SCENES:
        p1 = AUDIO_DIR / f"{scene}_p1.wav"
        if not p1.exists():
            text = NARRATION[scene]
            print(f"[tts-p1] {scene} ...")
            p1.write_bytes(tts(text, key, 1.0))
            print(f"          -> {p1.name} ({ffprobe_duration(p1):.2f}s)")

    # Second pass at a target pace that fills ~75% of each scene.
    for scene in SCENES:
        scene_mp4 = HERE / "media" / "videos" / "scenes" / "1080p30" / f"{scene}.mp4"
        dur = ffprobe_duration(scene_mp4)
        pace = target_pace(scene, dur)
        wav = AUDIO_DIR / f"{scene}.wav"
        pace_file = AUDIO_DIR / f"{scene}.pace"
        if wav.exists() and pace_file.exists() and pace_file.read_text().strip() == f"{pace:.3f}":
            print(f"[cache] {scene} (pace {pace:.2f})")
            continue
        print(f"[tts] {scene} pace={pace:.2f} (scene {dur:.1f}s) ...")
        wav.write_bytes(tts(NARRATION[scene], key, pace))
        (AUDIO_DIR / f"{scene}.pace").write_text(f"{pace:.3f}")
        print(f"      -> {wav.name} ({ffprobe_duration(wav):.2f}s)")

    # 2. fit each clip to its scene and concatenate
    print("fitting clips to scene durations ...")
    padded = []
    for scene in SCENES:
        src = AUDIO_DIR / f"{scene}.wav"
        scene_mp4 = HERE / "media" / "videos" / "scenes" / "1080p30" / f"{scene}.mp4"
        dur = ffprobe_duration(scene_mp4)
        aud = ffprobe_duration(src)
        out = AUDIO_DIR / f"{scene}_fitted.wav"
        if aud > dur * 1.02:
            atempo = aud / dur
            if atempo > 2.0:
                atempo = 2.0
            filt = f"[0:a]atempo={atempo:.4f},apad[a]"
            print(f"      {scene}: {aud:.2f}s > {dur:.2f}s -> atempo {atempo:.2f}")
        else:
            filt = "[0:a]apad[a]"
        subprocess.run([
            "ffmpeg", "-y", "-i", str(src), "-filter_complex", filt,
            "-map", "[a]", "-t", f"{dur:.3f}", "-ar", "44100", "-ac", "2",
            str(out),
        ], check=True, capture_output=True)
        padded.append(out)

    list_file = AUDIO_DIR / "concat.txt"
    list_file.write_text("".join(f"file '{p.name}'\n" for p in padded), encoding="utf-8")
    raw = AUDIO_DIR / "narration_raw.wav"
    subprocess.run([
        "ffmpeg", "-y", "-f", "concat", "-safe", "0", "-i", str(list_file),
        "-c", "copy", str(raw),
    ], check=True, capture_output=True)

    narration = AUDIO_DIR / "narration.wav"
    subprocess.run([
        "ffmpeg", "-y", "-i", str(raw),
        "-af", "loudnorm=I=-16:TP=-1.5:LRA=11",
        "-ar", "44100", "-ac", "2", str(narration),
    ], check=True, capture_output=True)

    # 3. mux
    print("muxing final video ...")
    subprocess.run([
        "ffmpeg", "-y", "-i", str(VIDEO), "-i", str(narration),
        "-c:v", "copy", "-c:a", "aac", "-b:a", "192k", "-shortest",
        str(OUT_VIDEO),
    ], check=True, capture_output=True)
    print(f"done -> {OUT_VIDEO} ({ffprobe_duration(OUT_VIDEO):.2f}s)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
