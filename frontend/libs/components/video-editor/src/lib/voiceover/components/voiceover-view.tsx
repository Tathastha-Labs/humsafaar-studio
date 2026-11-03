"use client";

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { PanelView } from "../../panels/assets/views/base-panel";
import { Button } from "../../components/ui/button";
import { Progress } from "../../components/ui/progress";
import { Slider } from "../../components/ui/slider";
import { Textarea } from "../../components/ui/textarea";
import { useEditorAdapters } from "../../EditorProvider";
import { EditorCore } from "../../core";
import { buildLibraryAudioElement } from "../../timeline/element-utils";
import { mediaTimeFromSeconds } from "../../wasm";
import type {
  VoiceoverClip,
  VoiceoverStatus,
  VoiceoverVoice,
} from "../../adapters";

const STATUS_POLL_MS = 1000;
const DEFAULT_SPEED = 1;
const SPEED_STEP = 0.05;
const DEFAULT_VOICE_ID = "af_heart";

export function VoiceoverView() {
  const { voiceover, toast } = useEditorAdapters();
  const [status, setStatus] = useState<VoiceoverStatus | null>(null);
  const [text, setText] = useState("");
  const [voiceId, setVoiceId] = useState(DEFAULT_VOICE_ID);
  const [speed, setSpeed] = useState(DEFAULT_SPEED);
  const [isGenerating, setIsGenerating] = useState(false);
  const [clip, setClip] = useState<VoiceoverClip | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  const textRef = useRef<HTMLTextAreaElement>(null);

  const refresh = useCallback(async () => {
    if (!voiceover) return;
    try {
      setStatus(await voiceover.getStatus());
      setLoadError(null);
    } catch (error) {
      setLoadError(error instanceof Error ? error.message : String(error));
    }
  }, [voiceover]);

  useEffect(() => {
    refresh();
  }, [refresh]);

  const isInstalling = status?.install.state === "installing";
  useEffect(() => {
    if (!isInstalling) return;
    const timer = setInterval(refresh, STATUS_POLL_MS);
    return () => clearInterval(timer);
  }, [isInstalling, refresh]);

  const voiceGroups = useMemo(() => groupByLanguage(status?.voices ?? []), [status]);

  if (!voiceover) {
    return (
      <PanelView title="Voiceover">
        <p className="text-muted-foreground p-3 text-sm">
          Voiceover is not available here.
        </p>
      </PanelView>
    );
  }

  const install = async () => {
    try {
      await voiceover.install();
    } catch (error) {
      toast.error(error instanceof Error ? error.message : String(error));
    }
    refresh();
  };

  const generate = async () => {
    if (!text.trim() || isGenerating) return;
    setIsGenerating(true);
    try {
      const result = await voiceover.generate({ text, voiceId, speed });
      setClip(result);
      await addToTimeline({ clip: result, name: nameFor(text) });
    } catch (error) {
      toast.error(error instanceof Error ? error.message : String(error));
    } finally {
      setIsGenerating(false);
    }
  };

  const addToTimeline = async ({ clip, name }: { clip: VoiceoverClip; name: string }) => {
    const editor = EditorCore.getInstance();
    try {
      const response = await fetch(clip.url);
      if (!response.ok) throw new Error(`Could not load the voiceover: ${response.statusText}`);
      const audioContext = new AudioContext();
      const buffer = await audioContext.decodeAudioData(await response.arrayBuffer());
      void audioContext.close();

      editor.timeline.insertElement({
        placement: { mode: "auto", trackType: "audio" },
        element: buildLibraryAudioElement({
          sourceUrl: clip.url,
          name,
          duration: mediaTimeFromSeconds({ seconds: clip.seconds }),
          startTime: editor.playback.getCurrentTime(),
          buffer,
        }),
      });
      toast.success("Voiceover added to the timeline");
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Could not add the voiceover to the timeline");
    }
  };

  const tooLong = status ? text.length > status.maxTextChars : false;

  return (
    <PanelView title="Voiceover" contentClassName="p-3">
      {loadError && <p className="text-destructive mb-3 text-sm">{loadError}</p>}

      {status && !status.installed && (
        <InstallCard status={status} onInstall={install} />
      )}

      {status?.installed && (
        <div className="flex flex-col gap-4">
          <div className="flex flex-col gap-1.5">
            <label className="text-muted-foreground text-xs" htmlFor="voiceover-script">
              Script
            </label>
            <Textarea
              id="voiceover-script"
              ref={textRef}
              value={text}
              onChange={(event) => setText(event.target.value)}
              placeholder="Type what the voice should say"
              className="min-h-32"
              aria-invalid={tooLong}
            />
            <span className={tooLong ? "text-destructive text-xs" : "text-muted-foreground text-xs"}>
              {text.length} / {status.maxTextChars} characters
            </span>
          </div>

          <div className="flex flex-col gap-1.5">
            <label className="text-muted-foreground text-xs" htmlFor="voiceover-voice">
              Voice
            </label>
            <select
              id="voiceover-voice"
              value={voiceId}
              onChange={(event) => setVoiceId(event.target.value)}
              className="border-input bg-accent h-9 rounded-md border px-2 text-sm outline-none"
            >
              {voiceGroups.map(([language, voices]) => (
                <optgroup key={language} label={language}>
                  {voices.map((voice) => (
                    <option key={voice.id} value={voice.id}>
                      {voice.label} ({voice.gender})
                    </option>
                  ))}
                </optgroup>
              ))}
            </select>
          </div>

          <div className="flex flex-col gap-1.5">
            <div className="flex items-center justify-between">
              <span className="text-muted-foreground text-xs">Speed</span>
              <span className="text-xs">{speed.toFixed(2)}x</span>
            </div>
            <Slider
              min={status.minSpeed}
              max={status.maxSpeed}
              step={SPEED_STEP}
              value={[speed]}
              onValueChange={([value]) => setSpeed(value)}
              aria-label="Speed"
            />
          </div>

          <Button onClick={generate} disabled={!text.trim() || tooLong || isGenerating}>
            {isGenerating ? "Making it..." : "Add to timeline"}
          </Button>

          {clip && (
            <div className="flex flex-col gap-2">
              <audio controls src={clip.url} className="w-full" />
              <Button
                variant="outline"
                onClick={() => addToTimeline({ clip, name: nameFor(text) })}
              >
                Add again at the playhead
              </Button>
            </div>
          )}
        </div>
      )}
    </PanelView>
  );
}

function InstallCard({ status, onInstall }: { status: VoiceoverStatus; onInstall: () => void }) {
  const install = status.install;
  const megabytes = Math.round(status.downloadBytes / 1_000_000);

  if (install.state === "installing") {
    return (
      <div className="flex flex-col gap-2">
        <span className="text-sm">{install.label}</span>
        <Progress value={Math.round((install.fraction ?? 0) * 100)} />
        {install.detail && (
          <span className="text-muted-foreground truncate text-xs">{install.detail}</span>
        )}
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-3">
      <p className="text-muted-foreground text-sm">
        Voiceover runs on this computer, so your script never leaves it. It needs a one-time
        download of about {megabytes} MB.
      </p>
      {install.state === "failed" && (
        <p className="text-destructive text-sm">{install.message}</p>
      )}
      <Button onClick={onInstall}>
        {install.state === "failed" ? "Try again" : "Download voices"}
      </Button>
    </div>
  );
}

function groupByLanguage(voices: VoiceoverVoice[]): [string, VoiceoverVoice[]][] {
  const groups = new Map<string, VoiceoverVoice[]>();
  for (const voice of voices) {
    const list = groups.get(voice.languageLabel) ?? [];
    list.push(voice);
    groups.set(voice.languageLabel, list);
  }
  return [...groups.entries()];
}

function nameFor(text: string): string {
  const firstWords = text.trim().split(/\s+/).slice(0, 5).join(" ");
  return `Voiceover: ${firstWords}`;
}
