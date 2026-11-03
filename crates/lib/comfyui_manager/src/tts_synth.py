"""Turns text into a WAV file with Kokoro. Reads one JSON request on stdin, prints one JSON result."""
import json
import sys

import soundfile
from kokoro_onnx import Kokoro

request = json.load(sys.stdin)
engine = Kokoro(request["model"], request["voices"])
samples, rate = engine.create(
    request["text"], voice=request["voice"], speed=request["speed"], lang=request["lang"]
)
soundfile.write(request["out"], samples, rate)
print(json.dumps({"seconds": len(samples) / rate, "sample_rate": rate}))
